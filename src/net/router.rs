use crate::bgp::session::{Session, SessionOpts};
use crate::bgp::timers::TimerConfig;
use crate::config::neighbor::NeighborConfig;
use crate::config::router::RouterConfig;
use crate::net::peer::Peer;
use crate::util::{LogLevel, log, log_banner, log_error, log_info};
use std::net::{TcpListener, TcpStream};

const MODULE: &str = "net.router";

pub struct RouterOpts {
    pub config_file_path: String,
    pub config: RouterConfig,
    pub full_listen_addr: String,
    pub log_level: LogLevel,
}

impl RouterOpts {
    pub fn new(config_file_path: String, log_level: u8) -> Result<RouterOpts, String> {
        let router_config: RouterConfig = RouterConfig::load(&config_file_path)?;
        let full_listen_addr: String = router_config.listen_addr.clone();
        let mapped_log_level: LogLevel = LogLevel::from_u8(log_level);
        log::set_log_level(mapped_log_level);
        Ok(RouterOpts {
            full_listen_addr,
            config: router_config,
            config_file_path,
            log_level: mapped_log_level,
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
        log_banner("Starting router with config");
        log_banner(format!("Config: {:?}", self.opts.config).as_str());
        let listener = TcpListener::bind(&self.opts.full_listen_addr).unwrap();
        log_info(
            MODULE,
            "Started listening on configured listen address",
            &[("listen_address", self.opts.full_listen_addr.to_string())],
        );

        // First check if there are non-passive neighbors to initiate connections to, add to our session list
        log_info(
            MODULE,
            "Initiating connections to configured neighbors",
            &[],
        );
        self.initiate_outbound_connections().unwrap();

        log_info(MODULE, "Listening for incoming connections", &[]);
        // Then, keep server open for incoming connections
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let peer_socket_addr = stream.peer_addr().unwrap();
                    log_info(
                        MODULE,
                        "Peer connected",
                        &[("peer", peer_socket_addr.to_string())],
                    );
                    let peer = Peer::new(stream, peer_socket_addr);
                    let session: Session =
                        Session::new(self.session_opts.clone(), self.timer_opts.clone(), peer);
                    spawn_session_thread(session);
                }
                Err(e) => {
                    log_error(
                        MODULE,
                        "Connection to peer failed",
                        &[("error", e.to_string())],
                    );
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
                    log_info(
                        MODULE,
                        "Successfully initiated outbound connection to peer",
                        &[("peer", neighbor.address.to_string())],
                    );
                    session.initiate()?;
                    spawn_session_thread(session);
                }
                Err(e) => {
                    log_error(
                        MODULE,
                        "Error initiating outbound connection",
                        &[
                            ("peer", neighbor.address.to_string()),
                            ("error", e.to_string()),
                        ],
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
