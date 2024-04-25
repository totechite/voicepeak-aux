mod voicepeak;
use anyhow::Result;
use clap::Parser;
use path_slash::PathBufExt;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use voicepeak::VoicePeak;

pub const VOICEPEAK_EXEC_PATH: &str = if cfg!(target_os = "windows") {
    "C:/'Program Files'/VOICEPEAK/voicepeak.exe"
} else if cfg!(target_os = "macos") {
    "/Applications/voicepeak.app/Contents/MacOS/voicepeak"
} else if cfg!(target_os = "linux") {
    ""
} else {
    panic!("non supported OS")
};

#[derive(Debug, Parser)]
#[clap(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
    about = env!("CARGO_PKG_DESCRIPTION"),
    disable_help_flag = false
)]
pub struct Args {
    /// Text to say
    #[clap(required = false, short_alias = 's', alias = "say", hide = true)]
    say: Option<String>,

    /// Text file to say
    #[clap(required = false, short = 't', long = "text", value_name = "File")]
    text: Option<String>,

    /// Path of output file
    #[clap(required = false, short = 'o', long = "out", value_name = "File")]
    out: Option<String>,

    /// Name of voice, check --list-narrator
    #[clap(required = false, short = 'n', long = "narrator", value_name = "Name")]
    narrator: Option<String>,

    /// Emotion expression, for example:
    /// happy=50,sad=50. Also check --list-emotion
    #[clap(required = false, short = 'e', long = "emotion", value_name = "Expr")]
    emotion: Option<String>,

    /// Print voice list
    #[clap(long = "list-narrator")]
    list_narrator: bool,

    /// Print emotion list for given voice
    #[clap(long = "list-emotion", value_name = "Narrator")]
    list_emotion: Option<String>,

    // /// Print help
    // #[clap(short = 'h', long = "help")]
    // help: bool,
    /// Speed (50 - 200)
    #[clap(required = false, long = "speed", value_name = "Value")]
    speed: Option<String>,

    /// Pitch (-300 - 300)
    #[clap(required = false, long = "pitch", value_name = "Value")]
    pitch: Option<String>,

    /// "Absolute Path of VOICEPEAK Executable File"
    #[clap(required = false, long = "aux-voicepeak-path", value_name = "PATH", default_value = VOICEPEAK_EXEC_PATH)]
    exec_path: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let voicepeak = VoicePeak::new(&args);

    if args.list_narrator {
        return voicepeak.run_list_narrator();
    }

    if let Some(_) = &args.list_emotion {
        return voicepeak.run_list_emotion();
    }

    // Note: textの取得
    if let (Some(text_path), Some(output_file_path)) = (&args.text, &args.out) {
        let mut reader = {
            let f = std::fs::File::open(text_path).expect("could not read file");
            std::io::BufReader::new(f)
        };

        let tmp_dir = tempdir()?;
        let tmp_dir_path = tmp_dir.path();

        let mut total_job_amount = 0;
        loop {
            let mut line = String::with_capacity(140 * 4);
            let read_byte = reader.read_line(&mut line)?;

            if read_byte == 0 {
                break;
            };

            total_job_amount += get_total_job_amount(line).unwrap();
        }

        print!("TotalJobAmount:{}", total_job_amount);

        let mut reader = {
            let f = std::fs::File::open(text_path).expect("could not read file");
            std::io::BufReader::new(f)
        };

        let mut count = 0;
        loop {
            let mut line = String::with_capacity(140 * 4);
            let read_byte = reader.read_line(&mut line)?;

            convert(&voicepeak, line, tmp_dir_path, count)?;

            if read_byte == 0 {
                break;
            };

            count += 1;
        }

        let mut output_file_path = PathBuf::from_slash(output_file_path);
        let mut current_dir_path = PathBuf::from_slash(std::env::current_dir()?.to_str().unwrap());
        if output_file_path.is_relative() {
            current_dir_path.push(output_file_path);
            output_file_path = current_dir_path;
        };
        merge_wav(tmp_dir_path, output_file_path.as_path())?;
    };

    Ok(())
}

fn convert(voicepeak: &VoicePeak, text: String, output_path: &Path, line_idx: u16) -> Result<()> {
    if text.chars().count() > 140 {
        // "．"で分割する
        let japanese_period_splited = text.split(&['。', '｡']).collect::<Vec<&str>>();
        for (japanese_period_idx, splited_text) in japanese_period_splited.into_iter().enumerate() {
            if splited_text.chars().count() <= 140 {
                // create dir
                let output_dir = output_path.join(line_idx.to_string());
                std::fs::create_dir_all(output_dir.clone()).unwrap_or(());

                let output_filename = output_dir.join(japanese_period_idx.to_string() + ".wav");

                voicepeak.exec(splited_text, output_filename.as_path())?;
            } else {
                // "、"で分割する
                let japanese_comma_splited = text.split(&['、', '､']).collect::<Vec<&str>>();
                for (japanese_comma_idx, splited_text) in japanese_comma_splited.iter().enumerate()
                {
                    if splited_text.chars().count() >= 140 {
                        panic!("140文字以上です")
                    }
                    // create dir
                    let output_dir = output_path
                        .join(line_idx.to_string())
                        .join(japanese_period_idx.to_string());
                    std::fs::create_dir_all(output_dir.clone()).unwrap_or(());

                    let output_filename = output_dir.join(japanese_comma_idx.to_string() + ".wav");

                    voicepeak.exec(splited_text, output_filename.as_path())?;
                }
            }
        }
    } else {
        // create dir
        let output_dir: &Path = output_path;
        std::fs::create_dir_all(output_dir).unwrap_or(());

        let output_filename = output_dir.join(line_idx.to_string() + ".wav");

        voicepeak.exec(text, output_filename.as_path())?;
    };

    Ok(())
}

fn get_total_job_amount(text: String) -> Result<usize> {
    let mut job_amount_counter = 0;
    if text.chars().count() > 140 {
        // "．"で分割する
        let japanese_period_splited = text.split(&['。', '｡']).collect::<Vec<&str>>();
        for (_, splited_text) in japanese_period_splited.into_iter().enumerate() {
            if splited_text.chars().count() <= 140 {
                job_amount_counter += 1;
            } else {
                // "、"で分割する
                let japanese_comma_splited = text.split(&['、', '､']).collect::<Vec<&str>>();
                for (_, splited_text) in japanese_comma_splited.iter().enumerate() {
                    if splited_text.chars().count() >= 140 {
                        panic!("140文字以上です")
                    }

                    job_amount_counter += 1;
                }
            }
        }
    } else {
        job_amount_counter += 1;
    };

    Ok(job_amount_counter)
}

fn merge_wav(merge_dir: &Path, output_file_name_path: &Path) -> Result<()> {
    for elem in std::fs::read_dir(merge_dir)?.filter_map(|e| e.ok()) {
        let elem_path = elem.path();
        if elem_path.is_dir() {
            let output_file_name_path =
                [elem_path.to_str().unwrap(), ".wav"].concat();
            merge_wav(
                elem_path.as_path(),
                Path::new(output_file_name_path.as_str()),
            )?;
        };
    }

    let file_count: i32 = std::fs::read_dir(merge_dir)?
        .into_iter()
        .filter_map(|e| e.ok())
        .fold(0, |mut file_count, f| {
            if f.path().is_file() {
                file_count += 1;
            };
            file_count
        });
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    std::fs::create_dir_all(output_file_name_path.parent().unwrap())?;
    let mut blob: hound::WavWriter<std::io::BufWriter<std::fs::File>> =
        hound::WavWriter::create(output_file_name_path, spec).unwrap();

    for file_number in 0..file_count {
        let file_path = merge_dir.join(file_number.to_string() + ".wav");

        // read from file
        let mut reader = match hound::WavReader::open(file_path.as_path()) {
            Ok(wav_reader) => wav_reader,
            Err(msg) => {
                panic!("{}", std::backtrace::Backtrace::capture());
            }
        };

        for sample in reader.samples::<i32>() {
            blob.write_sample(sample?)?
        }
    }

    blob.finalize().unwrap();

    Ok(())
}
