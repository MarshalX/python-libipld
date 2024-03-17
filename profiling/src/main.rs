use structopt::clap::arg_enum;
use structopt::StructOpt;

mod profiles;

arg_enum! {
    #[derive(Debug)]
    enum Profile {
        DecodeCar,
        EncodeDagCbor,
        DecodeDagCbor,
    }
}

#[derive(StructOpt, Debug)]
struct Opt {
    /// Profile to use
    #[structopt(possible_values = & Profile::variants(), case_insensitive = true)]
    profile: Profile,

    /// Number of profiling iterations
    #[structopt(long = "iterations", default_value = "100")]
    iterations: u64,
}

fn main() {
    let Opt {
        profile,
        iterations,
    } = Opt::from_args();
    match profile {
        Profile::DecodeCar => profiles::decode_car::exec(iterations),
        Profile::EncodeDagCbor => profiles::encode_dag_cbor::exec(iterations),
        Profile::DecodeDagCbor => profiles::decode_dag_cbor::exec(iterations),
    }
}
