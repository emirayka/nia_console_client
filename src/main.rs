mod repl;

use argh::FromArgs;

#[derive(FromArgs, Debug)]
#[argh(description = "...")]
struct CmdArgs {
    #[argh(option, description = "...")]
    port: usize,
}

fn main() {
    let cmd_args: CmdArgs = argh::from_env();

    println!("{:?}", cmd_args);

    repl::run(cmd_args.port);
}
