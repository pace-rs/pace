//! The three places pace keeps files: the global config file, the data
//! directory (`PACE_HOME`) and the machine directory.
//!
//! The data directory holds only data that may be synced. The machine
//! directory holds what must never be synced, such as this device's ID, so
//! it may not lie inside the data directory.

use std::{
    ffi::OsString,
    path::{Component, Path, PathBuf},
};

use pace_error::{ErrorKind, PaceError, PaceResult};

use crate::origin::Origin;

/// The name of the global config file.
pub const GLOBAL_CONFIG_FILE: &str = "pace.global.toml";

/// A path and where it comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located {
    /// The absolute path.
    pub path: PathBuf,
    /// Where it comes from.
    pub origin: Origin,
}

/// The resolved places.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    /// The global config file, `pace.global.toml`. It may not exist.
    pub config_file: Located,
    /// The data directory, `PACE_HOME`.
    pub data: Located,
    /// The machine directory, never synced: it holds this device's ID and,
    /// while a `pace setup` is interrupted, its `setup.toml`.
    pub machine: Located,
}

/// Paths given by flags or the environment, and portable mode.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inputs {
    /// `--config` or `PACE_CONFIG`.
    pub config_file: Option<Located>,
    /// `--home` or `PACE_HOME`.
    pub home: Option<Located>,
    /// `PACE_MACHINE_DIR`.
    pub machine: Option<Located>,
    /// The directory of the executable, if a `pace.global.toml` lies there.
    pub portable: Option<PathBuf>,
}

impl Inputs {
    /// Collect the given paths: a flag wins over its variable, an empty
    /// variable counts as unset, and relative paths start in the working
    /// directory `cwd`. A `pace.global.toml` beside the `executable` turns
    /// on portable mode.
    ///
    /// # Errors
    ///
    /// If a relative path cannot be made absolute.
    pub fn gather(
        flag_config: Option<&Path>,
        flag_home: Option<&Path>,
        env: impl Fn(&str) -> Option<OsString>,
        cwd: &Path,
        executable: Option<&Path>,
    ) -> PaceResult<Self> {
        let given = |flag: Option<(&Path, &'static str)>, var: &'static str| {
            let (path, origin) = match (flag, env(var).filter(|value| !value.is_empty())) {
                (Some((path, name)), _) => (path.to_path_buf(), Origin::Flag(name)),
                (None, Some(value)) => (PathBuf::from(value), Origin::Env(var)),
                (None, None) => return Ok(None),
            };
            absolute(&path, cwd).map(|path| Some(Located { path, origin }))
        };

        Ok(Self {
            config_file: given(flag_config.map(|path| (path, "--config")), "PACE_CONFIG")?,
            home: given(flag_home.map(|path| (path, "--home")), "PACE_HOME")?,
            machine: given(None, "PACE_MACHINE_DIR")?,
            portable: executable.and_then(portable_dir),
        })
    }
}

/// `path`, relative to `cwd` if it is relative.
fn absolute(path: &Path, cwd: &Path) -> PaceResult<PathBuf> {
    std::path::absolute(cwd.join(path)).map_err(|error| {
        PaceError::with_source(ErrorKind::Directories, "cannot find {path}", error)
            .attach_context("path", path.display().to_string())
            .attach_hint("give an absolute path")
    })
}

/// The absolute `path` as the file system sees it: its existing part with
/// symlinks, `.` and `..` resolved, the rest with `.` and `..` resolved by
/// name, since a part that doesn't exist yet holds no symlink.
fn normalised(path: &Path) -> PathBuf {
    let mut existing: Vec<Component<'_>> = path.components().collect();
    let mut missing = Vec::new();
    let mut real = loop {
        if let Ok(real) = existing.iter().collect::<PathBuf>().canonicalize() {
            break real;
        }
        match existing.pop() {
            Some(component) => missing.push(component),
            None => break PathBuf::new(),
        }
    };
    for component in missing.into_iter().rev() {
        match component {
            Component::ParentDir => _ = real.pop(),
            Component::CurDir => {}
            other => real.push(other),
        }
    }
    real
}

/// The directory of `executable`, if a `pace.global.toml` lies there.
fn portable_dir(executable: &Path) -> Option<PathBuf> {
    let dir = executable.parent()?;
    dir.join(GLOBAL_CONFIG_FILE)
        .is_file()
        .then(|| dir.to_path_buf())
}

/// The default directories of the operating system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Platform {
    config: PathBuf,
    data: PathBuf,
    machine: PathBuf,
}

impl Platform {
    /// Linux and other Unix systems: the XDG base directories.
    #[must_use]
    pub fn linux(config_home: &Path, data_home: &Path, state_home: &Path) -> Self {
        Self {
            config: config_home.join("pace"),
            data: data_home.join("pace"),
            machine: state_home.join("pace"),
        }
    }

    /// macOS: three subdirectories of `~/Library/Application Support/pace`,
    /// because macOS has no separate config or state directory.
    #[must_use]
    pub fn macos(application_support: &Path) -> Self {
        let base = application_support.join("pace");
        Self {
            config: base.join("config"),
            data: base.join("data"),
            machine: base.join("machine"),
        }
    }

    /// Windows: config and data in the roaming profile, the machine
    /// directory in the local one.
    #[must_use]
    pub fn windows(roaming: &Path, local: &Path) -> Self {
        let base = roaming.join("pace");
        Self {
            config: base.join("config"),
            data: base.join("data"),
            machine: local.join("pace"),
        }
    }

    fn portable(dir: &Path) -> Self {
        Self {
            config: dir.to_path_buf(),
            data: dir.join("data"),
            machine: dir.join("machine"),
        }
    }

    /// The directories of this system.
    ///
    /// # Errors
    ///
    /// If the home directory cannot be found.
    pub fn current() -> PaceResult<Self> {
        let base = directories::BaseDirs::new().ok_or_else(no_home)?;

        #[cfg(target_os = "macos")]
        let platform = Self::macos(base.data_dir());

        #[cfg(windows)]
        let platform = Self::windows(base.data_dir(), base.data_local_dir());

        #[cfg(not(any(target_os = "macos", windows)))]
        let platform = Self::linux(
            base.config_dir(),
            base.data_dir(),
            &base
                .state_dir()
                .map_or_else(|| base.home_dir().join(".local/state"), Path::to_path_buf),
        );

        Ok(platform)
    }
}

fn no_home() -> Box<PaceError> {
    PaceError::new(
        ErrorKind::Directories,
        "pace cannot find your home directory",
    )
    .attach_hint(
        "set PACE_HOME, PACE_MACHINE_DIR and PACE_CONFIG to tell pace where to keep its files",
    )
}

/// Resolve the three places. A given path wins over portable mode, portable
/// mode over the platform's defaults; `platform` is only asked if needed.
///
/// # Errors
///
/// If the platform's directories are needed but unknown, or the machine
/// directory lies inside the data directory.
pub fn resolve_dirs(
    inputs: Inputs,
    platform: impl FnOnce() -> PaceResult<Platform>,
) -> PaceResult<Dirs> {
    let Inputs {
        config_file,
        home,
        machine,
        portable,
    } = inputs;

    let defaults = match (&portable, &config_file, &home, &machine) {
        (Some(dir), ..) => Some((Platform::portable(dir), Origin::Portable)),
        (None, Some(_), Some(_), Some(_)) => None,
        (None, ..) => Some((platform()?, Origin::Default)),
    };
    let pick = |given: Option<Located>, default: fn(&Platform) -> PathBuf| match (given, &defaults)
    {
        (Some(given), _) => Ok(given),
        (None, Some((platform, origin))) => Ok(Located {
            path: default(platform),
            origin: origin.clone(),
        }),
        (None, None) => Err(PaceError::new(
            ErrorKind::Internal,
            "a directory has neither a given path nor a default",
        )
        .ask_report()),
    };

    let dirs = Dirs {
        config_file: pick(config_file, |platform| {
            platform.config.join(GLOBAL_CONFIG_FILE)
        })?,
        data: pick(home, |platform| platform.data.clone())?,
        machine: pick(machine, |platform| platform.machine.clone())?,
    };

    if normalised(&dirs.machine.path).starts_with(normalised(&dirs.data.path)) {
        return Err(PaceError::new(
            ErrorKind::Directories,
            "the machine directory {machine} lies inside the data directory {data}",
        )
        .attach_context("machine", dirs.machine.path.display().to_string())
        .attach_context("data", dirs.data.path.display().to_string())
        .attach_hint(
            "the data directory may be synced, and the machine directory holds this \
             device's ID; point PACE_MACHINE_DIR somewhere else",
        ));
    }

    Ok(dirs)
}

#[cfg(test)]
mod tests;
