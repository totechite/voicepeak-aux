use anyhow::Result;
use std::path::Path;
use std::process::Command;

use crate::{Args, VOICEPEAK_EXEC_PATH};

pub struct VoicePeak<'a> {
    shell: &'a str,
    subshell_option_str: &'a str,
    args: &'a Args,
}

// public
impl<'a> VoicePeak<'a> {
    pub fn new(args: &'a Args) -> Self {
        let (shell, subshell_option_str) = if cfg!(target_os = "windows") {
            ("powershell", "-c")
        } else if cfg!(target_os = "macos") {
            ("zsh", "-c")
        } else if cfg!(target_os = "linux") {
            ("sh", "-c")
        } else {
            panic!()
        };

        return VoicePeak {
            shell,
            subshell_option_str,
            args,
        };
    }

    pub fn exec(&self, text: impl AsRef<str>, out: &Path) -> Result<()> {
        let mut cmd = self.cmd_helper();

        if let Some(out_str) = out.to_str() {
            cmd.args(["--out", out_str]);
        };

        let quote_wrapped_text = ["'", text.as_ref(), "'"].concat();
        cmd.args(["--say", quote_wrapped_text.as_str()]);

        if let Some(narrator) = &self.args.narrator {
            let narrator = ["'", &narrator, "'"].concat();
            cmd.args(["--narrator", &narrator]);
        };

        if let Some(emotion) = &self.args.emotion {
            cmd.args(["--emotion", &emotion]);
        };

        if let Some(speed) = &self.args.speed {
            cmd.args(["--speed", &speed]);
        };

        if let Some(pitch) = &self.args.pitch {
            cmd.args(["--pitch", &pitch]);
        };

        let exit_status = cmd.output()?.status;

        Ok(())
    }

    pub fn run_list_narrator(&self) -> Result<()> {
        let mut cmd = self.cmd_helper();
        cmd.arg("--list-narrator");

        self.run_list_helper(&mut cmd)
    }

    pub fn run_list_emotion(&self) -> Result<()> {
        let mut cmd = self.cmd_helper();
        cmd.arg("--list-emotion");
        if let Some(narrator_name) = &self.args.list_emotion {
            cmd.arg(["'", narrator_name, "'"].concat());
        };

        self.run_list_helper(&mut cmd)
    }
}

// private
impl<'a> VoicePeak<'a> {
    fn cmd_helper(&self) -> Command {
        let mut cmd = Command::new(self.shell);
        cmd.args([self.subshell_option_str, VOICEPEAK_EXEC_PATH]);
        return cmd;
    }

    fn run_list_helper(&self, cmd: &mut Command) -> Result<()> {
        let stdout_text = {
            let stdout = String::from_utf8(cmd.output()?.stdout);
            let stderr = String::from_utf8(cmd.output()?.stderr);
            stdout.or(stderr)
        };
        match stdout_text {
            Ok(stdout_text) => println!("{}", stdout_text),

            Err(err) => eprintln!("Error:{}", err),
        };
        Ok(())
    }
}
