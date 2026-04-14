use stratsup::supervisor::Supervisor;

fn main() {
    let mut supervisor = Supervisor::new();
    loop {
        match supervisor.run_once() {
            Ok(()) => {
                if supervisor.shutdown_requested() {
                    break;
                }
            }
            Err(err) => {
                eprintln!("stratsup: {}", err);
                break;
            }
        }
    }
}
