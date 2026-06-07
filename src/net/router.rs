use crate::bgp::session::{Session, SessionOpts};
use crate::bgp::timers::TimerConfig;
use crate::config::neighbor::NeighborConfig;
use crate::config::router::RouterConfig;
use crate::net::peer::Peer;
use std::net::{TcpListener, TcpStream};

pub struct RouterOpts {
    pub config_file_path: String,
    pub config: RouterConfig,
    pub full_listen_addr: String,
}

impl RouterOpts {
    pub fn new(config_file_path: String) -> Result<RouterOpts, String> {
        let router_config: RouterConfig = RouterConfig::load(&config_file_path)?;
        let full_listen_addr: String = router_config.listen_addr.clone();
        Ok(RouterOpts {
            full_listen_addr,
            config: router_config,
            config_file_path,
        })
    }
}

pub struct Router {
    opts: RouterOpts,
    session_opts: SessionOpts,
    timer_opts: TimerConfig,
}

impl Router {
    pub fn new(opts: RouterOpts) -> Result<Router, String> {
        let session_opts =
            SessionOpts::new(opts.config.router_id.clone(), opts.config.local_as, false);
        let timer_opts =
            TimerConfig::new(opts.config.keepalive_interval, opts.config.hold_interval);
        Ok(Router {
            opts,
            session_opts,
            timer_opts,
        })
    }
    pub fn start(&mut self) {
        println!("----------- Starting router with config -----------");
        println!("{:?}", self.opts.config);
        println!("--------------------------------------------------");
        let listener = TcpListener::bind(&self.opts.full_listen_addr).unwrap();
        println!("Listening on {}", self.opts.full_listen_addr);

        // First check if there are non-passive neighbors to initiate connections to, add to our session list
        println!("Initiating outbound connections to configured neighbours");
        self.initiate_outbound_connections().unwrap();

        println!("Listen for incoming connections");
        // Then, keep server open for incoming connections
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let peer_socket_addr = stream.peer_addr().unwrap();
                    println!("Peer connected: {}", peer_socket_addr);
                    let peer = Peer::new(stream, peer_socket_addr);
                    let session: Session =
                        Session::new(self.session_opts.clone(), self.timer_opts.clone(), peer);
                    spawn_session_thread(session);
                }
                Err(e) => {
                    println!("Error while accepting: {}", e);
                }
            }
        }
    }

    fn initiate_outbound_connections(&self) -> Result<(), String> {
        let neighbors = &self.opts.config.neighbors;
        for neighbor in neighbors {
            if neighbor.passive {
                continue;
            }
            match self.initiate_outbound_connection(&neighbor) {
                Ok(mut session) => {
                    println!(
                        "Successfully established connection to peer {}",
                        neighbor.address
                    );
                    session.initiate()?;
                    spawn_session_thread(session);
                }
                Err(e) => {
                    println!(
                        "Failed to establish connection to peer {}, err: {}",
                        neighbor.address, e
                    );
                }
            }
        }
        Ok(())
    }

    fn initiate_outbound_connection(
        &self,
        neighbor_config: &NeighborConfig,
    ) -> Result<Session, String> {
        let stream = TcpStream::connect(&neighbor_config.address).map_err(|e| e.to_string())?;
        let peer_socket_addr = stream.peer_addr().map_err(|e| e.to_string())?;
        let peer = Peer::new(stream, peer_socket_addr);
        Ok(Session::new(
            self.session_opts.clone(),
            self.timer_opts.clone(),
            peer,
        ))
    }
}

fn spawn_session_thread(mut session: Session) {
    std::thread::spawn(move || {
        session.run();
    });
}
