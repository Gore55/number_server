use std::sync::Arc;
use std::collections::HashSet;
use tokio::fs::OpenOptions;
// use tokio::sync::{Mutex, mpsc::{UnboundedReceiver, UnboundedSender}};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use std::sync::Mutex;
use tokio::net::TcpListener;
use tokio::io::{AsyncBufReadExt, BufReader, BufWriter, AsyncWriteExt};
use std::net::{Ipv4Addr, SocketAddrV4};

pub struct AppStatus {
    total_unique: usize,
    n_duplicates: usize,
    n_new_unique: usize,
    n_clients: usize,
    terminate: bool,
}

impl AppStatus {
    pub fn new() -> Self {
        AppStatus {
            total_unique: 0,
            n_duplicates: 0,
            n_new_unique: 0,
            n_clients: 0,
            terminate: false,
        }
    }
    fn add_unique(&mut self) {
        self.n_new_unique += 1;
        self.total_unique += 1;
    }
    fn add_duplicate(&mut self) {
        self.n_new_unique += 1;
        self.n_duplicates += 1;
    }
    fn cero_(&mut self) {
        self.n_duplicates = 0;
        self.n_new_unique = 0;
    }
    fn status_print(&mut self) {
        println!("Received {} numbers - {} uniques | {} duplicates. Total: {} | {} clients connected", self.n_new_unique + self.n_duplicates, self.n_new_unique, self.n_duplicates, self.total_unique, self.n_clients);
        self.cero_();
    }
}

enum PossibleOutcome {
    Okay(u32),
    Bad,
    Terminate
}


fn process_line(line: &str) -> PossibleOutcome {
    match line {
        "terminate" => return PossibleOutcome::Terminate,
        _ => {
            if line.len() <= 9 {
                if let Ok(number) = line.parse::<u32>() {
                    return PossibleOutcome::Okay(number)
                } else {     
                    return PossibleOutcome::Bad
                };
            } else {
                return PossibleOutcome::Bad
            }
        }
    }
}

pub async fn start_listening(status: Arc<Mutex<AppStatus>>, tx: UnboundedSender<String>, port: u16)  -> Result<(), Box<dyn std::error::Error>>{
    
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port)).await?;
    println!("Now listening at: {}", listener.local_addr()?);
    let existing_numbers: Arc<Mutex<HashSet<u32>>> = Arc::new(Mutex::new(HashSet::new()));

    tokio::spawn(async move {

        loop {
            let terminate = status.lock().unwrap().terminate;
            if !terminate {
                match listener.accept().await {
                    Ok((mut peer, _)) => {
                        let n_connections = status.lock().unwrap().n_clients;
                        if n_connections > 4 {
                            println!("Too many clients");
                            if peer.shutdown().await.is_err() {
                                eprintln!("Failed to shutdown a client, breaking the loop")
                            };

                        } else {

                            status.lock().unwrap().n_clients += 1;
                            
                            let log_sender = tx.clone();
                            let ref_ex = existing_numbers.clone();
                            let ref_status = status.clone();
                            
                            tokio::spawn(async move {
                                
                                let mut reader = BufReader::new(peer.split().0);
                                let mut line = String::new();
                                
                                loop {
                                    if let Ok(size) = reader.read_line(&mut line).await {
                                        if size == 0 {
                                            if peer.shutdown().await.is_err() {
                                                eprintln!("Failed to shutdown a client, breaking the loop")
                                            };
                                            ref_status.lock().unwrap().n_clients -= 1;
                                            
                                            break
                                        };

                                        let send_line = line.trim_end().to_string();
                                        match process_line(&send_line) {

                                            PossibleOutcome::Okay(number) => {
                                                let mut locked_ex = ref_ex.lock().unwrap();

                                                if !locked_ex.contains(&number) {
                                                    ref_status.lock().unwrap().add_unique();
                                                    locked_ex.insert(number);
                                                    log_sender.send(send_line).expect("Broken pipe");
                                                } else {
                                                    ref_status.lock().unwrap().add_duplicate();
                                                }

                                            },
                                            PossibleOutcome::Bad => {
                                                if peer.shutdown().await.is_err() {
                                                    eprintln!("Failed to shutdown a client, breaking the loop")
                                                };
                                                ref_status.lock().unwrap().n_clients -= 1;   
                                                break
                                            },
                                            PossibleOutcome::Terminate => {
                                                ref_status.lock().unwrap().terminate = true;
                                            }
                                        }
                                    }
                                    if ref_status.lock().unwrap().terminate {
                                        if peer.shutdown().await.is_err() {
                                            eprintln!("Failed to shutdown a client, breaking the loop")
                                        };
                                        break
                                    }
                                    line.clear();
                                }
                            });
                        }
                    },
    
                    Err(e) => {
                        eprintln!("Error accepting new connection: {}", e);
                    }
                }
                    
            }
        }    
    });

    Ok(())

}

pub async fn start_logger(mut rx: UnboundedReceiver<String>) -> Result<(), Box<dyn std::error::Error>> {
    
    tokio::spawn(async move {
        let Ok(log_file) = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true) 
            .open("numbers.log").await else {
                panic!("Failed to open the file")
            };   
    
        let mut writer = BufWriter::new(log_file);
    
        while let Some(mut number) = rx.recv().await {
    
            number.push_str("\n");
            writer.write_all(number.as_bytes()).await.expect("Failed to write to buffer");
            writer.flush().await.expect("Failed to write to log file");
    
        }
    });

    Ok(())

}

pub async fn start_reporter(status: Arc<Mutex<AppStatus>>) -> Result<(), Box<dyn std::error::Error>> {
    
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
    
    loop {

        interval.tick().await;
        status.clone().lock().unwrap().status_print();

        if status.lock().unwrap().terminate {

            println!("Terminated by client");
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            std::process::exit(0);

        } 

    }

}
