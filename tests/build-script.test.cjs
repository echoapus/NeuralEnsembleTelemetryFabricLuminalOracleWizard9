'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {spawnSync} = require('node:child_process');
const root = path.resolve(__dirname, '..');
const script = fs.readFileSync(path.join(root, 'build.sh'), 'utf8');
const stub = `#!/bin/bash
set -eu
name=\${0##*/}
printf '%s' "$name" >> "$MOCK_LOG"
printf ' <%s>' "$@" >> "$MOCK_LOG"
printf '\\n' >> "$MOCK_LOG"
case "$name" in
    uname) if [[ $1 == -s ]]; then printf '%s\\n' "$MOCK_SYSTEM"; else printf 'x86_64\\n'; fi ;;
    id) printf '%s\\n' "$MOCK_UID" ;;
    sudo) exec "$@" ;;
    cc) /bin/cat >/dev/null; [[ $MOCK_CC == ready || -f $MOCK_STATE/cc-ready ]] ;;
    apt-get|dnf|yum|zypper|pacman)
        [[ $MOCK_PACKAGE_FAIL == no ]] || exit 42
        /usr/bin/touch "$MOCK_STATE/cc-ready"
        ;;
    rustc)
        if [[ -f $MOCK_STATE/rust-ready ]]; then printf 'rustc 1.93.1 (mock)\\n';
        elif [[ $MOCK_RUST != missing ]]; then printf 'rustc %s (mock)\\n' "$MOCK_RUST";
        else exit 127; fi
        ;;
    cargo)
        if [[ $1 == --version ]]; then printf 'cargo 1.93.1\\n'; exit; fi
        [[ $MOCK_BUILD_FAIL == no ]] || exit 43
        /bin/mkdir -p target/release
        /bin/cat > target/release/nfv9gen <<'BINARY'
#!/bin/bash
printf 'nfv9gen' >> "$MOCK_LOG"
printf ' <%s>' "$@" >> "$MOCK_LOG"
printf '\\n' >> "$MOCK_LOG"
[[ $MOCK_SMOKE_FAIL == no ]]
BINARY
        /bin/chmod +x target/release/nfv9gen
        ;;
    rustup)
        if [[ $1 == toolchain ]]; then /usr/bin/touch "$MOCK_STATE/rust-ready";
        else shift 2; exec "$@"; fi
        ;;
    curl)
        [[ $MOCK_DOWNLOAD_FAIL == no ]] || exit 44
        while [[ $1 != -o ]]; do shift; done
        /bin/cat > "$2" <<'INSTALLER'
#!/bin/sh
printf 'installer\\n' >> "$MOCK_LOG"
/bin/cp "$MOCK_STATE/rustup-stub" "$CARGO_HOME/bin/rustup"
/usr/bin/touch "$MOCK_STATE/rust-ready"
INSTALLER
        ;;
esac
`;
let count = 0;
function check(options = {}) {
    const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'nfv9gen-build-test-'));
    try {
        const project = path.join(temp, 'project with spaces'), bin = path.join(temp, 'bin');
        fs.mkdirSync(project); fs.mkdirSync(bin);
        fs.mkdirSync(path.join(temp, 'cargo-home', 'bin'), {recursive:true});
        const release = path.join(temp, 'os-release');
        fs.writeFileSync(release, `ID=${options.id || 'ubuntu'}\nID_LIKE="${options.like || ''}"\nPRETTY_NAME="Test Linux"\n`);
        // Replace the system metadata path only; no test switches in the shipped script.
        fs.writeFileSync(path.join(project, 'build.sh'), script.replaceAll('/etc/os-release', release));
        fs.copyFileSync(path.join(root, 'Cargo.toml'), path.join(project, 'Cargo.toml'));
        fs.copyFileSync(path.join(root, 'Cargo.lock'), path.join(project, 'Cargo.lock'));
        for (const name of ['dirname','mktemp','rm','sed']) fs.symlinkSync('/usr/bin/'+name, path.join(bin,name));
        fs.symlinkSync('/bin/sh', path.join(bin,'sh'));
        const commands = ['uname','id','cc','curl','cargo','rustc','apt-get','zypper','pacman'];
        if (options.sudo !== false) commands.push('sudo');
        commands.push(options.yum ? 'yum' : 'dnf');
        if (options.rustup !== false) commands.push('rustup');
        for (const name of commands) fs.writeFileSync(path.join(bin,name),stub,{mode:0o755});
        fs.writeFileSync(path.join(temp,'rustup-stub'),stub,{mode:0o755});
        const log = path.join(temp,'commands');
        const result = spawnSync('/bin/bash',[path.join(project,'build.sh'),...(options.args || [])],{
            cwd:temp, encoding:'utf8', env:{...process.env, PATH:bin, CARGO_HOME:path.join(temp,'cargo-home'), SUDO_USER:options.sudoUser || '',
                MOCK_LOG:log, MOCK_STATE:temp, MOCK_SYSTEM:options.system || 'Linux', MOCK_UID:options.uid || '1000',
                MOCK_CC:options.cc || 'ready', MOCK_RUST:options.rust || '1.93.1', MOCK_PACKAGE_FAIL:options.packageFail ? 'yes':'no',
                MOCK_BUILD_FAIL:options.buildFail ? 'yes':'no', MOCK_SMOKE_FAIL:options.smokeFail ? 'yes':'no', MOCK_DOWNLOAD_FAIL:options.downloadFail ? 'yes':'no'},
        });
        if (options.fail) assert(result.status !== null && result.status !== 0, result.stdout+'\n'+result.stderr);
        else assert.equal(result.status,0,result.stdout+'\n'+result.stderr);
        const calls = fs.existsSync(log) ? fs.readFileSync(log,'utf8') : '';
        if (options.includes) assert.match(calls,options.includes);
        if (options.excludes) assert.doesNotMatch(calls,options.excludes);
        if (!options.fail) {
            assert.match(calls,/cargo <build> <--release> <--locked> <--target-dir>/);
            assert.match(calls,/nfv9gen <validate> <examples\/basic.json>/);
            assert.match(calls,/nfv9gen <preview> <examples\/basic.json> <--kind> <data> <--hex>/);
            assert.match(result.stdout,/Build complete:/);
        } else assert.doesNotMatch(result.stdout,/Build complete:/);
        count++;
    } finally { fs.rmSync(temp,{recursive:true,force:true}); }
}
check({excludes:/sudo|apt-get|installer|toolchain/});
for (const [id,like,expected] of [
    ['ubuntu','debian',/sudo <apt-get> <install> <-y> <build-essential>/],
    ['linuxmint','ubuntu debian',/apt-get <update>/],
    ['fedora','',/dnf <install> <-y> <gcc> <glibc-devel>/],
    ['rocky','rhel centos fedora',/dnf <install>/],
    ['opensuse-leap','suse opensuse',/zypper <--non-interactive> <install>/],
    ['arch','',/pacman <-S> <--needed> <--noconfirm>/],
]) check({id,like,cc:'missing',includes:expected});
check({id:'centos',cc:'missing',yum:true,includes:/yum <install>/});
check({cc:'missing',uid:'0',excludes:/sudo/});
check({rust:'1.84.1',includes:/rustup <toolchain> <install> <stable>/});
check({rust:'1.85.0',excludes:/toolchain|installer/});
check({rust:'missing',rustup:false,includes:/installer\nrustup <run> <stable> <rustc>/});
check({id:'unknown',rust:'missing',includes:/rustup <toolchain> <install> <stable>/,excludes:/sudo|apt-get/});
check({rust:'missing',rustup:false,downloadFail:true,fail:true,excludes:/installer|cargo <build>/});
check({system:'Darwin',fail:true,excludes:/sudo|cargo/});
check({id:'unknown',cc:'missing',fail:true,excludes:/sudo|apt-get <install>|cargo <build>/});
check({cc:'missing',sudo:false,fail:true,excludes:/apt-get|cargo <build>/});
check({sudoUser:'alice',fail:true,excludes:/sudo|cargo/});
check({args:['--unknown'],fail:true,excludes:/sudo|cargo/});
check({cc:'missing',packageFail:true,fail:true,excludes:/cargo <build>/});
check({buildFail:true,fail:true,excludes:/nfv9gen </});
check({smokeFail:true,fail:true});
console.log(`Passed ${count} build-script checks: distro packages, privilege boundaries, toolchain reuse/install, paths with spaces, and failure handling. No real packages or toolchains were installed.`);
