#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate semver;
extern crate toml;

mod cargo;
mod cli;
mod docker;
mod errors;
mod extensions;
mod file;
mod id;
mod volume;

use std::process::ExitStatus;
use std::{env, process};

use toml::{Parser, Value};

use cargo::Root;
use errors::*;

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq)]
pub enum Host {
    Other,

    // OSX
    X86_64AppleDarwin,

    // Linux
    X86_64UnknownLinuxGnu,
}

impl<'a> From<&'a str> for Host {
    fn from(s: &str) -> Host {
        match s {
            "x86_64-apple-darwin" => Host::X86_64AppleDarwin,
            "x86_64-unknown-linux-gnu" => Host::X86_64UnknownLinuxGnu,
            _ => Host::Other,
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, PartialEq)]
pub enum Target {
    Custom { triple: String },

    // Other built-in
    Other,

    // OSX
    I686AppleDarwin,
    X86_64AppleDarwin,

    // Android
    ArmLinuxAndroideabi,
    Armv7LinuxAndroideabi,
    Aarch64LinuxAndroid,
    I686LinuxAndroid,
    X86_64LinuxAndroid,

    // Linux
    Aarch64UnknownLinuxGnu,
    ArmUnknownLinuxGnueabi,
    ArmUnknownLinuxMusleabi,
    Armv7UnknownLinuxGnueabihf,
    Armv7UnknownLinuxMusleabihf,
    I586UnknownLinuxGnu,
    I686UnknownLinuxGnu,
    I686UnknownLinuxMusl,
    Mips64UnknownLinuxGnuabi64,
    Mips64elUnknownLinuxGnuabi64,
    MipsUnknownLinuxGnu,
    MipselUnknownLinuxGnu,
    Powerpc64UnknownLinuxGnu,
    Powerpc64leUnknownLinuxGnu,
    PowerpcUnknownLinuxGnu,
    S390xUnknownLinuxGnu,
    Sparc64UnknownLinuxGnu,
    X86_64UnknownLinuxGnu,
    X86_64UnknownLinuxMusl,

    // *BSD
    I686UnknownFreebsd,
    X86_64UnknownDragonfly,
    X86_64UnknownFreebsd,
    X86_64UnknownNetbsd,

    // Solaris / illumos
    Sparcv9SunSolaris,
    X86_64SunSolaris,

    // Windows
    X86_64PcWindowsGnu,
    I686PcWindowsGnu,

    // Emscripten
    AsmjsUnknownEmscripten,
    Wasm32UnknownEmscripten,

    // Bare metal
    Thumbv6mNoneEabi,
    Thumbv7emNoneEabi,
    Thumbv7emNoneEabihf,
    Thumbv7mNoneEabi,
}

impl Target {
    fn is_bare_metal(&self) -> bool {
        match *self {
            Target::Thumbv6mNoneEabi |
            Target::Thumbv7emNoneEabi |
            Target::Thumbv7emNoneEabihf |
            Target::Thumbv7mNoneEabi => true,
            _ => false,
        }
    }

    fn is_builtin(&self) -> bool {
        match *self {
            Target::Custom { .. } => false,
            _ => true,
        }
    }

    fn triple(&self) -> &str {
        use Target::*;

        match *self {
            Custom { ref triple } => triple,
            Other => unreachable!(),

            Aarch64LinuxAndroid => "aarch64-linux-android",
            Aarch64UnknownLinuxGnu => "aarch64-unknown-linux-gnu",
            ArmLinuxAndroideabi => "arm-linux-androideabi",
            ArmUnknownLinuxGnueabi => "arm-unknown-linux-gnueabi",
            ArmUnknownLinuxMusleabi => "arm-unknown-linux-musleabi",
            Armv7LinuxAndroideabi => "armv7-linux-androideabi",
            Armv7UnknownLinuxGnueabihf => "armv7-unknown-linux-gnueabihf",
            Armv7UnknownLinuxMusleabihf => "armv7-unknown-linux-musleabihf",
            AsmjsUnknownEmscripten => "asmjs-unknown-emscripten",
            I586UnknownLinuxGnu => "i586-unknown-linux-gnu",
            I686AppleDarwin => "i686-apple-darwin",
            I686LinuxAndroid => "i686-linux-android",
            I686PcWindowsGnu => "i686-pc-windows-gnu",
            I686UnknownFreebsd => "i686-unknown-freebsd",
            I686UnknownLinuxGnu => "i686-unknown-linux-gnu",
            I686UnknownLinuxMusl => "i686-unknown-linux-musl",
            Mips64UnknownLinuxGnuabi64 => "mips64-unknown-linux-gnuabi64",
            Mips64elUnknownLinuxGnuabi64 => "mips64el-unknown-linux-gnuabi64",
            MipsUnknownLinuxGnu => "mips-unknown-linux-gnu",
            MipselUnknownLinuxGnu => "mipsel-unknown-linux-gnu",
            Powerpc64UnknownLinuxGnu => "powerpc64-unknown-linux-gnu",
            Powerpc64leUnknownLinuxGnu => "powerpc64le-unknown-linux-gnu",
            PowerpcUnknownLinuxGnu => "powerpc-unknown-linux-gnu",
            S390xUnknownLinuxGnu => "s390x-unknown-linux-gnu",
            Sparc64UnknownLinuxGnu => "sparc64-unknown-linux-gnu",
            Sparcv9SunSolaris => "sparcv9-sun-solaris",
            Thumbv6mNoneEabi => "thumbv6m-none-eabi",
            Thumbv7emNoneEabi => "thumbv7em-none-eabi",
            Thumbv7emNoneEabihf => "thumbv7em-none-eabihf",
            Thumbv7mNoneEabi => "thumbv7m-none-eabi",
            Wasm32UnknownEmscripten => "wasm32-unknown-emscripten",
            X86_64AppleDarwin => "x86_64-apple-darwin",
            X86_64PcWindowsGnu => "x86_64-pc-windows-gnu",
            X86_64LinuxAndroid => "x86_64-linux-android",
            X86_64SunSolaris => "x86_64-sun-solaris",
            X86_64UnknownDragonfly => "x86_64-unknown-dragonfly",
            X86_64UnknownFreebsd => "x86_64-unknown-freebsd",
            X86_64UnknownLinuxGnu => "x86_64-unknown-linux-gnu",
            X86_64UnknownLinuxMusl => "x86_64-unknown-linux-musl",
            X86_64UnknownNetbsd => "x86_64-unknown-netbsd",
        }
    }

    fn needs_xargo(&self) -> bool {
        self.is_bare_metal() || !self.is_builtin()
    }
}

impl Target {
    fn from(triple: &str) -> Target {
        use Target::*;

        match triple {
            "aarch64-linux-android" => Aarch64LinuxAndroid,
            "aarch64-unknown-linux-gnu" => Aarch64UnknownLinuxGnu,
            "arm-linux-androideabi" => ArmLinuxAndroideabi,
            "arm-unknown-linux-gnueabi" => ArmUnknownLinuxGnueabi,
            "arm-unknown-linux-musleabi" => ArmUnknownLinuxMusleabi,
            "armv7-linux-androideabi" => Armv7LinuxAndroideabi,
            "armv7-unknown-linux-gnueabihf" => Armv7UnknownLinuxGnueabihf,
            "armv7-unknown-linux-musleabihf" => Armv7UnknownLinuxMusleabihf,
            "asmjs-unknown-emscripten" => AsmjsUnknownEmscripten,
            "i586-unknown-linux-gnu" => I586UnknownLinuxGnu,
            "i686-apple-darwin" => I686AppleDarwin,
            "i686-linux-android" => I686LinuxAndroid,
            "i686-pc-windows-gnu" => I686PcWindowsGnu,
            "i686-unknown-freebsd" => I686UnknownFreebsd,
            "i686-unknown-linux-gnu" => I686UnknownLinuxGnu,
            "i686-unknown-linux-musl" => I686UnknownLinuxMusl,
            "mips-unknown-linux-gnu" => MipsUnknownLinuxGnu,
            "mips64-unknown-linux-gnuabi64" => Mips64UnknownLinuxGnuabi64,
            "mips64el-unknown-linux-gnuabi64" => Mips64elUnknownLinuxGnuabi64,
            "mipsel-unknown-linux-gnu" => MipselUnknownLinuxGnu,
            "powerpc-unknown-linux-gnu" => PowerpcUnknownLinuxGnu,
            "powerpc64-unknown-linux-gnu" => Powerpc64UnknownLinuxGnu,
            "powerpc64le-unknown-linux-gnu" => Powerpc64leUnknownLinuxGnu,
            "s390x-unknown-linux-gnu" => S390xUnknownLinuxGnu,
            "sparc64-unknown-linux-gnu" => Sparc64UnknownLinuxGnu,
            "sparcv9-sun-solaris" => Sparcv9SunSolaris,
            "thumbv6m-none-eabi" => Thumbv6mNoneEabi,
            "thumbv7em-none-eabi" => Thumbv7emNoneEabi,
            "thumbv7em-none-eabihf" => Thumbv7emNoneEabihf,
            "thumbv7m-none-eabi" => Thumbv7mNoneEabi,
            "wasm32-unknown-emscripten" => Wasm32UnknownEmscripten,
            "x86_64-apple-darwin" => X86_64AppleDarwin,
            "x86_64-linux-android" => X86_64LinuxAndroid,
            "x86_64-pc-windows-gnu" => X86_64PcWindowsGnu,
            "x86_64-sun-solaris" => X86_64SunSolaris,
            "x86_64-unknown-dragonfly" => X86_64UnknownDragonfly,
            "x86_64-unknown-freebsd" => X86_64UnknownFreebsd,
            "x86_64-unknown-linux-gnu" => X86_64UnknownLinuxGnu,
            "x86_64-unknown-linux-musl" => X86_64UnknownLinuxMusl,
            "x86_64-unknown-netbsd" => X86_64UnknownNetbsd,
            _ => Custom { triple: triple.to_owned() },
        }
    }
}

impl From<Host> for Target {
    fn from(host: Host) -> Target {
        match host {
            Host::X86_64UnknownLinuxGnu => Target::X86_64UnknownLinuxGnu,
            Host::X86_64AppleDarwin => Target::X86_64AppleDarwin,
            Host::Other => unreachable!(),
        }
    }
}

pub fn main() {
    fn show_backtrace() -> bool {
        env::var("RUST_BACKTRACE").as_ref().map(|s| &s[..]) == Ok("1")
    }

    match run() {
        Err(e) => {
            eprintln!("error: {}", e);

            for e in e.iter().skip(1) {
                eprintln!("caused by: {}", e);
            }

            if show_backtrace() {
                if let Some(backtrace) = e.backtrace() {
                    eprintln!("{:?}", backtrace);
                }
            } else {
                eprintln!("note: run with `RUST_BACKTRACE=1` for a backtrace");
            }

            process::exit(1)
        }
        Ok(status) => {
            if !status.success() {
                process::exit(status.code().unwrap_or(1))
            }
        }
    }
}

fn run() -> Result<ExitStatus> {
    let args = cli::parse();


    if args.all.iter().any(|a| a == "--version" || a == "-V") &&
       args.subcommand.is_none() {
        println!(concat!("cross ", env!("CARGO_PKG_VERSION"), "{}"),
                 include_str!(concat!(env!("OUT_DIR"), "/commit-info.txt")));
    }

    let verbose =
        args.all.iter().any(|a| a == "--verbose" || a == "-v" || a == "-vv");

    if let Some(root) = cargo::root()? {
        let target = args.target.unwrap_or_else(|| Target::X86_64UnknownLinuxGnu);
        let toml = toml(&root)?;

        let uses_xargo = if let Some(toml) = toml.as_ref() {
            toml.xargo(&target)?
        } else {
            None
        }
        .unwrap_or_else(|| target.needs_xargo());

        let vol_info = volume::populate_volume(&target,
                                args.toolchain,
                                toml.as_ref(),
                                uses_xargo,
                                verbose
        )?;

        return docker::run(&target,
                           &args.all,
                           &root,
                           toml.as_ref(),
                           uses_xargo,
                           verbose,
                           &vol_info);

    }

    eprintln!("Warning! Failed to `cross`. Passing through to Cargo...");
    cargo::run(&args.all, verbose)
}

/// Parsed `Cross.toml`
pub struct Toml {
    table: Value,
}

impl Toml {
    /// Returns the `target.{}.image` part of `Cross.toml`
    pub fn image(&self, target: &Target) -> Result<Option<&str>> {
        let triple = target.triple();

        if let Some(value) = self.table
            .lookup(&format!("target.{}.image", triple)) {
            Ok(Some(value.as_str()
                .ok_or_else(|| {
                    format!("target.{}.image must be a string", triple)
                })?))
        } else {
            Ok(None)
        }
    }

    /// Returns the `target.{}.toolchain` part of `Cross.toml`
    pub fn toolchain(&self, target: &Target) -> Result<Option<&str>> {
        let triple = target.triple();

        if let Some(value) = self.table
            .lookup(&format!("target.{}.toolchain", triple)) {
            Ok(Some(value.as_str()
                .ok_or_else(|| {
                    format!("target.{}.toolchain must be a string", triple)
                })?))
        } else {
            Ok(None)
        }
    }

    /// Returns the `build.image` or the `target.{}.xargo` part of `Cross.toml`
    pub fn xargo(&self, target: &Target) -> Result<Option<bool>> {
        let triple = target.triple();

        if let Some(value) = self.table.lookup("build.xargo") {
            return Ok(Some(value.as_bool()
                .ok_or_else(|| "build.xargo must be a boolean")?));
        }

        if let Some(value) = self.table
            .lookup(&format!("target.{}.xargo", triple)) {
            Ok(Some(value.as_bool()
                .ok_or_else(|| {
                    format!("target.{}.xargo must be a boolean", triple)
                })?))
        } else {
            Ok(None)
        }
    }

    /// Returns the list of environment variables to pass through for `target`,
    /// including variables specified under `build` and under `target`.
    pub fn env_passthrough(&self, target: &Target) -> Result<Vec<&str>> {
        let mut bwl = self.build_env_passthrough()?;
        let mut twl = self.target_env_passthrough(target)?;
        bwl.extend(twl.drain(..));

        Ok(bwl)
    }

    /// Returns the `build.env.passthrough` part of `Cross.toml`
    fn build_env_passthrough(&self) -> Result<Vec<&str>> {
        match self.table.lookup("build.env.passthrough") {
            Some(&Value::Array(ref vec)) => {
                if vec.iter().any(|val| val.as_str().is_none()) {
                    bail!("every build.env.passthrough element must be a string");
                }
                Ok(vec.iter().map(|val| val.as_str().unwrap()).collect())
            },
            _ => Ok(Vec::new()),
        }
    }

    /// Returns the `target.<triple>.env.passthrough` part of `Cross.toml` for `target`.
    fn target_env_passthrough(&self, target: &Target) -> Result<Vec<&str>> {
        let triple = target.triple();

        let key = format!("target.{}.env.passthrough", triple);

        match self.table.lookup(&key) {
            Some(&Value::Array(ref vec)) => {
                if vec.iter().any(|val| val.as_str().is_none()) {
                    bail!("every {} element must be a string", key);
                }
                Ok(vec.iter().map(|val| val.as_str().unwrap()).collect())
            },
            _ => Ok(Vec::new()),
        }
    }
}

/// Parses the `Cross.toml` at the root of the Cargo project (if any)
fn toml(root: &Root) -> Result<Option<Toml>> {
    let path = root.path().join("Cross.toml");

    if path.exists() {
        Ok(Some(Toml {
            table: Value::Table(Parser::new(&file::read(&path)?).parse()
                .ok_or_else(|| {
                    format!("couldn't parse {} as TOML", path.display())
                })?),
        }))
    } else {
        Ok(None)
    }
}
