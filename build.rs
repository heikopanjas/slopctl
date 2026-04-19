use std::{env, fs, path::PathBuf};

#[path = "src/cli.rs"] mod cli;

fn main()
{
    let profile = env::var("PROFILE").unwrap_or_default();
    if profile != "release"
    {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let man_dir = out_dir.join("man");
    fs::create_dir_all(&man_dir).unwrap();

    let cmd = cli::Cli::command();

    // Main man page
    let man = clap_mangen::Man::new(cmd.clone());
    let mut buffer: Vec<u8> = Vec::new();
    man.render(&mut buffer).unwrap();
    fs::write(man_dir.join("slopctl.1"), &buffer).unwrap();

    // Per-subcommand man pages
    for subcommand in cmd.get_subcommands()
    {
        let name = format!("slopctl-{}", subcommand.get_name());
        let filename = format!("{}.1", name);
        let leaked: &'static str = name.leak();
        let subcmd = subcommand.clone().name(leaked);
        let man = clap_mangen::Man::new(subcmd);
        let mut buffer: Vec<u8> = Vec::new();
        man.render(&mut buffer).unwrap();
        fs::write(man_dir.join(&filename), &buffer).unwrap();
    }
}
