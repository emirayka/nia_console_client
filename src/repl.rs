use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use rustyline::error::ReadlineError;
use rustyline::Editor;

use nia_protocol_rust::*;
use protobuf::Message;

const HISTORY_FILE_NAME: &'static str = ".nia-console-client.history";

fn get_history_file_path() -> Option<String> {
    match dirs::home_dir() {
        Some(dir) => match dir.as_path().join(HISTORY_FILE_NAME).to_str() {
            Some(s) => Some(s.to_string()),
            _ => None,
        },
        _ => None,
    }
}

fn make_handshake_request() -> Request {
    let handshake_request = HandshakeRequest::new();

    let mut request = Request::new();
    request.set_handshake_request(handshake_request);

    request
}

fn make_execute_code_request(code: String) -> Request {
    let mut execute_code_request = ExecuteCodeRequest::new();

    execute_code_request.set_code(protobuf::Chars::from(code));

    let mut request = Request::new();
    request.set_execute_code_request(execute_code_request);

    request
}

fn print_handshake_response(response: HandshakeResponse) {
    let mut response = response;

    if response.has_error_result() {
        let error_result = response.take_error_result();

        println!("Handshake with the server was failure :(");
        println!("Message: {}", error_result.get_message());
    } else {
        let success_result = response.take_success_result();

        println!("Handshake with the server was successful :3");
        println!("Server version: {}", success_result.get_version());
        println!("Server info: {}", success_result.get_info());
    }
}

fn print_execute_code_response(response: ExecuteCodeResponse) {
    let mut response = response;

    if response.has_success_result() {
        let success_result = response.take_success_result();

        println!("Execution result is success :)");
        println!("Message: {}", success_result.get_execution_result());
    } else if response.has_error_result() {
        let error_result = response.take_error_result();

        println!("Execution result is error :(");
        println!("Message: {}", error_result.get_message());
    } else if response.has_failure_result() {
        let failure_result = response.take_failure_result();

        println!("Execution result is failure :O");
        println!("Message: {}", failure_result.get_message());
    }
}

fn print_unknown_response(response: Response) {
    println!("Got unknown response :O");

    if response.has_execute_code_response() {
        println!("Response type is execute code response. That is strange for sure :/");
    } else if response.has_handshake_response() {
        println!("Response type is handshake response. That is strange for sure :/");
    } else if response.has_get_devices_response() {
        println!("Response type is get devices response. That is weird...");
    } else if response.has_get_device_info_response() {
        println!("Response type is get device info response. That is weird...");
    } else {
        println!("Got completely unexpected response!!! Server are u ok??!");
    }
}

fn connect_to_server(port: usize) -> (mpsc::Sender<String>, mpsc::Receiver<Response>) {
    let (string_sender, string_receiver) = mpsc::channel::<String>();
    let (response_sender, response_receiver) = mpsc::channel();

    let string_receiver = Arc::new(Mutex::new(string_receiver));
    let response_sender = Arc::new(Mutex::new(response_sender));

    let server_path = format!("ws://127.0.0.1:{}", port);

    thread::spawn(move || {
        ws::connect(server_path, move |out| {
            let string_receiver = Arc::clone(&string_receiver);
            let response_sender = Arc::clone(&response_sender);

            // send handshake request
            let handshake_request = make_handshake_request();

            let bytes = handshake_request
                .write_to_bytes()
                .expect("Failure while serializing request");

            match out.send(bytes) {
                Ok(_) => {}
                Err(error) => {
                    println!("Error during send message to the server occured...");
                    println!("{:?}", error);
                }
            }

            move |msg| {
                let string_receiver = string_receiver.lock().unwrap();
                let response_sender = response_sender.lock().unwrap();

                // send response
                let response = match msg {
                    ws::Message::Binary(bytes) => {
                        let mut response = Response::new();

                        response
                            .merge_from_bytes(&bytes)
                            .expect("Failure while merging response from the server.");

                        response
                    }
                    ws::Message::Text(_) => {
                        println!("Got text message instead of binary :/");
                        println!("The server probably is not that we was looking for...");
                        println!("Ignoring...");

                        return Ok(());
                    }
                };

                match response_sender.send(response) {
                    Ok(_) => {}
                    Err(_) => {
                        println!("Response channel is ded now :(");
                        println!("No more responses can be sent.");

                        match out.close(ws::CloseCode::Normal) {
                            Ok(_) => {
                                println!("Successfully closed connection.");
                            }
                            Err(_) => {
                                println!("Connection was closed with an error.");
                            }
                        };

                        println!("Connection to the server is closed now.");
                    }
                }

                // send execute code request
                let string = match string_receiver.recv() {
                    Ok(s) => s,
                    Err(_) => {
                        println!("String channel is closed now.");
                        println!("Cannot get message anymore..");
                        println!("Closing connection to the server.");

                        match out.close(ws::CloseCode::Normal) {
                            Ok(_) => {
                                println!("Successfully closed connection.");
                            }
                            Err(_) => {
                                println!("Connection was closed with an error.");
                            }
                        };

                        println!("Connection to the server is closed now.");

                        return Ok(());
                    }
                };

                let execute_code_request = make_execute_code_request(string);
                let bytes = execute_code_request
                    .write_to_bytes()
                    .expect("Failure while serializing request.");

                out.send(bytes)
                    .expect("Failure while sending request to the server.");

                Ok(())
            }
        })
        .unwrap();
    });

    (string_sender, response_receiver)
}

pub fn run(port: usize) -> Result<(), std::io::Error> {
    let history_file = get_history_file_path();

    let mut rl = Editor::<()>::new();

    if let Some(history) = &history_file {
        match rl.load_history(history) {
            Ok(_) => {}
            Err(_) => {
                println!("History file can't be constructed.");
            }
        };
    } else {
        println!("History file can't be constructed.");
    }

    let (string_sender, response_receiver) = connect_to_server(port);

    match response_receiver.recv() {
        Ok(response) => {
            let mut response = response;

            if response.has_handshake_response() {
                print_handshake_response(response.take_handshake_response());
            } else {
                println!("Seems that we connected to the wrong server...");
                println!("Exiting...");
                return Ok(());
            }
        }
        _ => {
            println!("Response channel is ded at the stage of handshake.");
            println!("Exiting...");
            return Ok(());
        }
    };

    loop {
        let readline = rl.readline(">> ");

        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());

                match string_sender.send(line) {
                    Ok(_) => {}
                    _ => {
                        println!(
                            "Finally, because string channel is ded the app will be terminated..."
                        );
                        break;
                    }
                };

                let mut response = match response_receiver.recv() {
                    Ok(response) => response,
                    _ => {
                        println!("Finally, because response channel is ded the app will be terminated...");
                        break;
                    }
                };

                if response.has_execute_code_response() {
                    print_execute_code_response(response.take_execute_code_response());
                } else {
                    print_unknown_response(response);
                }
            }
            Err(ReadlineError::Interrupted) => {
                // break;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    if let Some(history) = &history_file {
        rl.save_history(history)
            .expect(&format!("Failure saving history at: {}", history));
    }

    Ok(())
}
