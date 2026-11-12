use std::{
    collections::HashMap,
    io::{self, Read},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Result, ensure};

use crate::{
    config::{Flow, Values},
    raw::checksum,
};

pub const HEADER_LENGTH: usize = 20;
pub const IPFIX_HEADER_LENGTH: usize = 16;
pub const DEFAULT_PACKET_SIZE: usize = 1400;

#[derive(Debug)]
pub struct Packet {
    pub bytes: Vec<u8>,
    pub template_id: u16,
    pub data_records: u16,
    pub random_fields: Vec<RandomField>,
    pub record_length: usize,
    /// Record offsets of IPv4 headers inside frame sections; see `fix_checksums`.
    pub ipv4_headers: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct RandomField {
    pub offset: usize,
    pub min: u32,
    pub max: u32,
    pub length: usize,
    /// When set, min..=max indexes these length-byte values instead of being the value.
    pub pool: Option<Arc<[u8]>>,
}

impl Packet {
    pub fn is_ipfix(&self) -> bool {
        self.bytes[1] == 10
    }

    /// Offset of the first record: the message header plus the 4-byte set header.
    fn records_at(&self) -> usize {
        4 + if self.is_ipfix() {
            IPFIX_HEADER_LENGTH
        } else {
            HEADER_LENGTH
        }
    }

    /// Recomputes the IPv4 header checksum of every generated frame section.
    fn fix_checksums(&mut self) {
        for record in 0..self.data_records as usize {
            for offset in &self.ipv4_headers {
                let at = self.records_at() + record * self.record_length + offset;
                self.bytes[at + 10..at + 12].fill(0);
                let sum = checksum(&self.bytes[at..at + 20], 0);
                self.bytes[at + 10..at + 12].copy_from_slice(&sum.to_be_bytes());
            }
        }
    }

    pub fn randomize_fields(&mut self, random: &mut impl Read) -> io::Result<()> {
        for field in &self.random_fields {
            // Pool indexes use 4 random bytes; numbers use their own field width.
            let draw = if field.pool.is_some() {
                4
            } else {
                field.length
            };
            let width = u64::from(field.max) - u64::from(field.min) + 1;
            let domain = 1u64 << (draw * 8);
            let bound = domain - domain % width;
            for record in 0..self.data_records as usize {
                // Rejection sampling avoids bias when the range does not divide the domain.
                let value = loop {
                    let mut bytes = [0; 4];
                    random.read_exact(&mut bytes[4 - draw..])?;
                    let value = u64::from(u32::from_be_bytes(bytes));
                    if value < bound {
                        break (u64::from(field.min) + value % width) as u32;
                    }
                };
                let at = self.records_at() + record * self.record_length + field.offset;
                let target = &mut self.bytes[at..at + field.length];
                match &field.pool {
                    Some(pool) => {
                        let from = value as usize * field.length;
                        target.copy_from_slice(&pool[from..from + field.length]);
                    }
                    None => target.copy_from_slice(&value.to_be_bytes()[4 - field.length..]),
                }
            }
        }
        self.fix_checksums();
        Ok(())
    }

    pub fn stamp(
        &mut self,
        source_id: u32,
        sequence: u32,
        uptime: Duration,
        now: SystemTime,
    ) -> Result<()> {
        let unix = now.duration_since(UNIX_EPOCH)?;
        if self.is_ipfix() {
            // RFC 7011: export time, then a sequence counting data records, then the domain.
            self.bytes[4..8].copy_from_slice(&(unix.as_secs() as u32).to_be_bytes());
            self.bytes[8..12].copy_from_slice(&sequence.to_be_bytes());
            self.bytes[12..16].copy_from_slice(&source_id.to_be_bytes());
            return Ok(());
        }
        self.bytes[4..8].copy_from_slice(&(uptime.as_millis() as u32).to_be_bytes());
        self.bytes[8..12].copy_from_slice(&(unix.as_secs() as u32).to_be_bytes());
        self.bytes[12..16].copy_from_slice(&sequence.to_be_bytes());
        self.bytes[16..20].copy_from_slice(&source_id.to_be_bytes());
        Ok(())
    }
}

#[derive(Debug)]
pub struct Plan {
    pub source_id: u32,
    pub templates: Vec<Packet>,
    pub data: Vec<Packet>,
    pub records: usize,
}

fn push_u16(bytes: &mut Vec<u8>, value: usize) {
    bytes.extend_from_slice(&(value as u16).to_be_bytes());
}

fn packet(
    ipfix: bool,
    flowset_id: u16,
    template_id: u16,
    body: &[u8],
    count: u16,
    data_records: u16,
    padding: usize,
) -> Packet {
    let header = if ipfix {
        IPFIX_HEADER_LENGTH
    } else {
        HEADER_LENGTH
    };
    let total = header + 4 + body.len() + padding;
    let mut bytes = Vec::with_capacity(total);
    if ipfix {
        // IPFIX carries the message length where v9 carries the record count.
        bytes.extend_from_slice(&10u16.to_be_bytes());
        push_u16(&mut bytes, total);
    } else {
        bytes.extend_from_slice(&9u16.to_be_bytes());
        bytes.extend_from_slice(&count.to_be_bytes());
    }
    bytes.resize(header, 0);
    push_u16(&mut bytes, flowset_id as usize);
    push_u16(&mut bytes, 4 + body.len() + padding);
    bytes.extend_from_slice(body);
    bytes.resize(bytes.len() + padding, 0);
    Packet {
        bytes,
        template_id,
        data_records,
        random_fields: Vec::new(),
        record_length: 0,
        ipv4_headers: Vec::new(),
    }
}

fn data_padding(length: usize, record_length: usize) -> usize {
    let padding = (4 - length % 4) % 4;
    // RFC 3954 erratum 2096: padding must not look like another short record.
    if padding < record_length { padding } else { 0 }
}

pub fn build(source_id: u32, flows: &[Flow], limit: usize) -> Result<Plan> {
    encode(source_id, flows, limit, false)
}

pub fn build_ipfix(source_id: u32, flows: &[Flow], limit: usize) -> Result<Plan> {
    encode(source_id, flows, limit, true)
}

fn encode(source_id: u32, flows: &[Flow], limit: usize, ipfix: bool) -> Result<Plan> {
    let start = 4 + if ipfix {
        IPFIX_HEADER_LENGTH
    } else {
        HEADER_LENGTH
    };
    ensure!(
        (64..=65507).contains(&limit),
        "packet-size must be within 64..=65507 (UDP payload bytes)"
    );
    let mut plan = Plan {
        source_id,
        templates: Vec::new(),
        data: Vec::new(),
        records: 0,
    };
    let mut templates: HashMap<u16, &Flow> = HashMap::new();
    for flow in flows {
        let mut random_fields = Vec::new();
        let mut offset = 0;
        for column in &flow.columns {
            match &column.values {
                Values::RandomField { min, max, length } => random_fields.push(RandomField {
                    offset,
                    min: *min,
                    max: *max,
                    length: *length,
                    pool: None,
                }),
                Values::RandomFile { bytes, length } => random_fields.push(RandomField {
                    offset,
                    min: 0,
                    max: (bytes.len() / length - 1) as u32,
                    length: *length,
                    pool: Some(bytes.clone()),
                }),
                _ => {}
            }
            offset += column.spec.length;
        }
        if let Some(previous) = templates.insert(flow.template_id, flow) {
            ensure!(
                flow.same_template(previous),
                "Template {}: shared template structures do not match",
                flow.template_id
            );
        } else {
            let mut descriptors = Vec::new();
            let fields = flow.template_columns().count();
            push_u16(&mut descriptors, flow.template_id as usize);
            if flow.scope_count == 0 {
                push_u16(&mut descriptors, fields);
            } else if ipfix {
                // IPFIX counts fields; v9 gives the byte length of each descriptor group.
                push_u16(&mut descriptors, fields);
                push_u16(&mut descriptors, flow.scope_count);
            } else {
                push_u16(&mut descriptors, flow.scope_count * 4);
                push_u16(&mut descriptors, (fields - flow.scope_count) * 4);
            }
            for column in flow.template_columns() {
                push_u16(&mut descriptors, column.spec.id as usize);
                push_u16(
                    &mut descriptors,
                    if column.spec.variable {
                        65535
                    } else {
                        column.spec.length
                    },
                );
            }
            let padding = (4 - descriptors.len() % 4) % 4;
            let template = packet(
                ipfix,
                // Template / options template set: 0 / 1 in v9, 2 / 3 in IPFIX.
                u16::from(ipfix) * 2 + u16::from(flow.scope_count > 0),
                flow.template_id,
                &descriptors,
                1,
                0,
                padding,
            );
            ensure!(
                template.bytes.len() <= limit,
                "Template {} description needs {} bytes, exceeding packet-size {limit}",
                flow.template_id,
                template.bytes.len()
            );
            plan.templates.push(template);
        }

        let mut capacity = (limit - start) / flow.record_length;
        while capacity > 0 {
            let size = capacity * flow.record_length;
            if start + size + data_padding(size, flow.record_length) <= limit {
                break;
            }
            capacity -= 1;
        }
        ensure!(
            capacity > 0,
            "Template {} record of {} bytes does not fit in packet-size {limit}",
            flow.template_id,
            flow.record_length
        );
        let mut body = Vec::with_capacity(capacity * flow.record_length);
        let mut indexes = vec![0usize; flow.columns.len()];
        let mut count = 0u16;
        for record in 0..flow.record_count {
            for (column, index) in flow.columns.iter().zip(&indexes) {
                column.values.append(*index, &mut body);
            }
            count += 1;
            if count as usize == capacity || record + 1 == flow.record_count {
                let padding = data_padding(body.len(), flow.record_length);
                let mut data = packet(
                    ipfix,
                    flow.template_id,
                    flow.template_id,
                    &body,
                    count,
                    count,
                    padding,
                );
                data.random_fields = random_fields.clone();
                data.record_length = flow.record_length;
                data.ipv4_headers = flow.ipv4_headers.clone();
                data.fix_checksums();
                plan.data.push(data);
                body.clear();
                count = 0;
            }
            for (index, column) in indexes.iter_mut().zip(&flow.columns).rev() {
                *index += 1;
                if *index < column.values.count() {
                    break;
                }
                *index = 0;
            }
        }
        plan.records += flow.record_count;
    }
    Ok(plan)
}
