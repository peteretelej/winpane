mod dispatch;
mod protocol;
mod util;

use std::io::{self, BufRead, BufWriter, Write};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use dispatch::{Dispatcher, event_to_json};
use protocol::{ErrorResponse, Notification, Response, parse_request, serialize_line};

fn write_line(stdout: &Arc<Mutex<BufWriter<io::Stdout>>>, line: &str) {
    let mut out = stdout.lock().expect("stdout mutex poisoned");
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

fn handle_request(dispatcher: &mut Dispatcher, line: &str) -> String {
    match parse_request(line) {
        Err(err_resp) => serialize_line(&err_resp),
        Ok(req) => match dispatcher.dispatch(&req.method, &req.params) {
            Ok(result) => serialize_line(&Response::ok(req.id, result)),
            Err((code, message)) => serialize_line(&ErrorResponse::new(req.id, code, message)),
        },
    }
}

#[allow(clippy::print_stderr)]
fn main() {
    let version = env!("CARGO_PKG_VERSION");
    eprintln!("winpane-host v{version}");

    let mut dispatcher = match Dispatcher::new() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("fatal: {e}");
            std::process::exit(1);
        }
    };

    let stdout: Arc<Mutex<BufWriter<io::Stdout>>> =
        Arc::new(Mutex::new(BufWriter::new(io::stdout())));

    // Stdin reader thread
    let (tx, rx) = mpsc::channel::<Option<String>>();
    thread::spawn(move || {
        let stdin = io::stdin();
        let reader = stdin.lock();
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    if tx.send(Some(l)).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = tx.send(None); // EOF
    });

    // Main dispatch + event loop
    loop {
        let mut did_work = false;

        // Check for incoming requests
        match rx.try_recv() {
            Ok(Some(line)) => {
                did_work = true;
                if line.trim().is_empty() {
                    // Skip empty lines
                } else {
                    let response = handle_request(&mut dispatcher, &line);
                    write_line(&stdout, &response);
                }
            }
            Ok(None) => break, // EOF
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => break,
        }

        // Poll for engine events
        while let Some(event) = dispatcher.poll_event() {
            did_work = true;
            let params = event_to_json(&event, dispatcher.surface_id_map());
            let notification = serialize_line(&Notification::event(params));
            write_line(&stdout, &notification);
        }

        if !did_work {
            thread::sleep(Duration::from_millis(5));
        }
    }
}
