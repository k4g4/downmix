use anyhow::{ensure, Context};
use clap::Parser;
use std::{path::PathBuf, process::Command};
use tracing::{info, Level};

#[derive(Parser)]
/// Downmixes a video file's audio into stereo sound if it isn't already
struct Args {
    input_path: PathBuf,

    output_path: PathBuf,

    #[arg(short, long)]
    /// Suppress output
    quiet: bool,

    #[arg(short, long)]
    /// Overwrite an existing file
    force: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_max_level(if args.quiet { Level::WARN } else { Level::INFO })
        .with_level(false)
        .with_target(false)
        .without_time()
        .init();

    ensure!(
        args.input_path.try_exists()?,
        "'{}' does not exist",
        args.input_path.display(),
    );

    ensure!(
        args.input_path.is_file(),
        "'{}' is not a file",
        args.input_path.display(),
    );

    if !args.force {
        ensure!(
            !args.output_path.try_exists()?,
            "'{}' already exists. Use --force to overwrite.",
            args.output_path.display()
        );
    }

    let ffprobe_args = [
        args.input_path
            .to_str()
            .context(format!("invalid path '{}'", args.input_path.display()))?,
        "-show_streams",
        "-loglevel",
        "error",
        "-print_format",
        "json",
    ];

    let output = Command::new("ffprobe").args(ffprobe_args).output()?;

    ensure!(
        output.stderr.is_empty(),
        "Error from ffprobe:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json = serde_json::from_slice::<serde_json::Value>(&output.stdout)?;
    let streams = json
        .get("streams")
        .and_then(|streams| streams.as_array())
        .context("invalid json")?;

    let mut too_many_channels = false;
    for stream in streams {
        if let Some(channels) = stream.get("channels") {
            let channels = channels.as_i64().context("invalid metadata value")?;

            info!(
                "Found {channels} channels for '{}'",
                args.input_path.display()
            );

            too_many_channels |= channels > 2;
        }
    }

    if too_many_channels {
        info!(
            "Downmixing '{}' to '{}'",
            args.input_path.display(),
            args.output_path.display()
        );

        downmix(args)
    } else {
        println!(
            "File '{}' does not need to be downmixed.",
            args.input_path.display()
        );

        Ok(())
    }
}

fn downmix(args: Args) -> anyhow::Result<()> {
    let ffmpeg_args = [
        "-i",
        args.input_path.to_str().unwrap(),
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-c:v",
        "copy",
        "-ac",
        "2",
        args.output_path
            .to_str()
            .context(format!("invalid path '{}'", args.output_path.display()))?,
    ];

    let output = Command::new("ffmpeg").args(ffmpeg_args).output()?;

    ensure!(
        output.stderr.is_empty(),
        "Error from ffmpeg:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    info!("Successfully downmixed to '{}'", args.output_path.display());

    Ok(())
}
