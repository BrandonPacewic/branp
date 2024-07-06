use std::env;

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    for arg in args {
        println!("{arg:?}");
    }
}
