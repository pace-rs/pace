use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use pace_store::testing::scratch_dir;

use pace_error::{ErrorKind, PaceResult};

use crate::dirs::{Dirs, Inputs, Located, Platform, no_home, resolve_dirs};
use crate::origin::Origin;

fn path(text: &str) -> PathBuf {
    PathBuf::from(text)
}

fn linux() -> Platform {
    Platform::linux(
        Path::new("/home/me/.config"),
        Path::new("/home/me/.local/share"),
        Path::new("/home/me/.local/state"),
    )
}

fn located(text: &str, origin: Origin) -> Located {
    Located {
        path: path(text),
        origin,
    }
}

fn paths(dirs: &Dirs) -> [&Path; 3] {
    [
        dirs.config_file.path.as_path(),
        dirs.data.path.as_path(),
        dirs.machine.path.as_path(),
    ]
}

#[test]
fn linux_follows_the_xdg_directories() -> PaceResult<()> {
    let dirs = resolve_dirs(Inputs::default(), || Ok(linux()))?;

    assert_eq!(
        paths(&dirs),
        [
            Path::new("/home/me/.config/pace/pace.global.toml"),
            Path::new("/home/me/.local/share/pace"),
            Path::new("/home/me/.local/state/pace"),
        ]
    );
    assert_eq!(
        [
            dirs.config_file.origin,
            dirs.data.origin,
            dirs.machine.origin
        ],
        [Origin::Default, Origin::Default, Origin::Default]
    );
    Ok(())
}

// directories returns one path for config and data on macOS and has no
// state directory, so pace adds its own subdirectories.
#[test]
fn macos_keeps_three_subdirectories_of_application_support() -> PaceResult<()> {
    let dirs = resolve_dirs(Inputs::default(), || {
        Ok(Platform::macos(Path::new(
            "/Users/me/Library/Application Support",
        )))
    })?;

    assert_eq!(
        paths(&dirs),
        [
            Path::new("/Users/me/Library/Application Support/pace/config/pace.global.toml"),
            Path::new("/Users/me/Library/Application Support/pace/data"),
            Path::new("/Users/me/Library/Application Support/pace/machine"),
        ]
    );
    Ok(())
}

#[test]
fn windows_keeps_the_machine_directory_out_of_the_roaming_profile() -> PaceResult<()> {
    let dirs = resolve_dirs(Inputs::default(), || {
        Ok(Platform::windows(
            Path::new("C:/Users/me/AppData/Roaming"),
            Path::new("C:/Users/me/AppData/Local"),
        ))
    })?;

    assert_eq!(
        paths(&dirs),
        [
            Path::new("C:/Users/me/AppData/Roaming/pace/config/pace.global.toml"),
            Path::new("C:/Users/me/AppData/Roaming/pace/data"),
            Path::new("C:/Users/me/AppData/Local/pace"),
        ]
    );
    Ok(())
}

#[test]
fn given_paths_win_over_the_defaults() -> PaceResult<()> {
    let inputs = Inputs {
        config_file: Some(located("/c/pace.toml", Origin::Flag("--config"))),
        home: Some(located("/sync/pace", Origin::Env("PACE_HOME"))),
        machine: Some(located("/m", Origin::Env("PACE_MACHINE_DIR"))),
        portable: None,
    };

    let dirs = resolve_dirs(inputs, || Ok(linux()))?;

    assert_eq!(
        paths(&dirs),
        [
            Path::new("/c/pace.toml"),
            Path::new("/sync/pace"),
            Path::new("/m")
        ]
    );
    assert_eq!(dirs.data.origin, Origin::Env("PACE_HOME"));
    Ok(())
}

#[test]
fn portable_mode_keeps_everything_next_to_the_executable() -> PaceResult<()> {
    let inputs = Inputs {
        portable: Some(path("/media/stick/pace")),
        ..Inputs::default()
    };

    let dirs = resolve_dirs(inputs, || Ok(linux()))?;

    assert_eq!(
        paths(&dirs),
        [
            Path::new("/media/stick/pace/pace.global.toml"),
            Path::new("/media/stick/pace/data"),
            Path::new("/media/stick/pace/machine"),
        ]
    );
    assert_eq!(dirs.data.origin, Origin::Portable);
    Ok(())
}

#[test]
fn given_paths_win_over_portable_mode() -> PaceResult<()> {
    let inputs = Inputs {
        home: Some(located("/sync/pace", Origin::Flag("--home"))),
        portable: Some(path("/media/stick/pace")),
        ..Inputs::default()
    };

    let dirs = resolve_dirs(inputs, || Ok(linux()))?;

    assert_eq!(dirs.data.path, path("/sync/pace"));
    assert_eq!(dirs.machine.path, path("/media/stick/pace/machine"));
    Ok(())
}

#[test]
fn the_platform_is_not_asked_when_every_path_is_given() -> PaceResult<()> {
    let inputs = Inputs {
        config_file: Some(located("/c/pace.toml", Origin::Flag("--config"))),
        home: Some(located("/sync/pace", Origin::Flag("--home"))),
        machine: Some(located("/m", Origin::Env("PACE_MACHINE_DIR"))),
        portable: None,
    };

    let dirs = resolve_dirs(inputs, || Err(no_home()))?;

    assert_eq!(dirs.machine.path, path("/m"));
    Ok(())
}

#[test]
fn missing_home_directory_says_which_variables_help() {
    let error = resolve_dirs(Inputs::default(), || Err(no_home())).err();

    assert_eq!(
        error.as_ref().and_then(|error| error.hint()),
        Some(
            "set PACE_HOME, PACE_MACHINE_DIR and PACE_CONFIG to tell pace where to keep its files"
        )
    );
}

#[test]
fn the_machine_directory_must_not_be_inside_the_data_directory() {
    for machine in ["/sync/pace", "/sync/pace/machine"] {
        let inputs = Inputs {
            home: Some(located("/sync/pace", Origin::Env("PACE_HOME"))),
            machine: Some(located(machine, Origin::Env("PACE_MACHINE_DIR"))),
            ..Inputs::default()
        };

        let error = resolve_dirs(inputs, || Ok(linux())).err();

        assert_eq!(
            error.as_ref().map(|error| error.kind()),
            Some(ErrorKind::Directories),
            "{machine}"
        );
    }
}

#[test]
fn machine_directory_reached_through_parent_lies_inside_data_directory() {
    let inputs = Inputs {
        home: Some(located("/sync/pace", Origin::Env("PACE_HOME"))),
        machine: Some(located(
            "/sync/work/../pace/machine",
            Origin::Env("PACE_MACHINE_DIR"),
        )),
        ..Inputs::default()
    };

    let error = resolve_dirs(inputs, || Ok(linux())).err();

    assert_eq!(
        error.as_ref().map(|error| error.kind()),
        Some(ErrorKind::Directories)
    );
}

#[cfg(unix)]
#[test]
fn machine_directory_reached_through_symlink_lies_inside_data_directory()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = scratch_dir()?;
    let home = dir.path().join("home");
    std::fs::create_dir(&home)?;
    std::os::unix::fs::symlink(&home, dir.path().join("link"))?;
    let inputs = Inputs {
        home: Some(Located {
            path: home,
            origin: Origin::Env("PACE_HOME"),
        }),
        machine: Some(Located {
            path: dir.path().join("link/machine"),
            origin: Origin::Env("PACE_MACHINE_DIR"),
        }),
        ..Inputs::default()
    };

    let error = resolve_dirs(inputs, || Ok(linux())).err();

    assert_eq!(
        error.as_ref().map(|error| error.kind()),
        Some(ErrorKind::Directories)
    );
    Ok(())
}

#[test]
fn machine_directory_beside_data_directory_is_kept() -> PaceResult<()> {
    let inputs = Inputs {
        home: Some(located("/sync/pace", Origin::Env("PACE_HOME"))),
        machine: Some(located(
            "/sync/pace/../pace-machine",
            Origin::Env("PACE_MACHINE_DIR"),
        )),
        ..Inputs::default()
    };

    let dirs = resolve_dirs(inputs, || Ok(linux()))?;

    assert_eq!(dirs.machine.path, path("/sync/pace/../pace-machine"));
    Ok(())
}

fn no_env(_: &str) -> Option<OsString> {
    None
}

#[test]
fn relative_path_starts_in_given_working_directory() -> PaceResult<()> {
    let env = |name: &str| (name == "PACE_HOME").then(|| OsString::from("data"));

    let inputs = Inputs::gather(None, None, env, Path::new("/home/me/work"), None)?;

    assert_eq!(
        inputs.home,
        Some(located("/home/me/work/data", Origin::Env("PACE_HOME")))
    );
    Ok(())
}

#[test]
fn executable_beside_global_config_is_portable() -> Result<(), Box<dyn std::error::Error>> {
    let dir = scratch_dir()?;
    std::fs::write(dir.path().join("pace.global.toml"), "")?;
    let executable = dir.path().join("pace");

    let inputs = Inputs::gather(None, None, no_env, Path::new("/"), Some(&executable))?;

    assert_eq!(inputs.portable, Some(dir.path().to_path_buf()));
    Ok(())
}

#[test]
fn executable_without_global_config_is_not_portable() -> Result<(), Box<dyn std::error::Error>> {
    let dir = scratch_dir()?;
    let executable = dir.path().join("pace");

    let inputs = Inputs::gather(None, None, no_env, Path::new("/"), Some(&executable))?;

    assert_eq!(inputs.portable, None);
    Ok(())
}
