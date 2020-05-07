mod repl;

use argh::FromArgs;

#[derive(FromArgs, Debug)]
#[argh(description = "...")]
struct CmdArgs {
    #[argh(option, description = "...")]
    port: usize,
}

fn main() -> Result<(), std::io::Error> {
    let cmd_args: CmdArgs = argh::from_env();

    repl::run(cmd_args.port)?;

    Ok(())
}
