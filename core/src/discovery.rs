use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::time;
use serde::{Serialize, Deserialize};

use crate::proto::vdfs::NodeInfo;

const DISCOVERY_MULTICAST_ADDR: &str = "239.255.0.1:5353";
const DISCOVERY_PORT: u16 = 5353;
const DISCOVERY_INTERVAL: Duration = Duration::from_secs(60);
const PACKET_SIZE: usize = 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryMessage {
    Announce {
        node_id: String,
        name: String,
        host: String,
        port: i32,
        services: Vec<String>,
    },
    Query {
        node_id: String,
        network: String,
    },
    Response {
        node_id: String,
        name: String,
        host: String,
        port: i32,
        services: Vec<String>,
    },
}

enum DiscoveryCommand {
    Stop,
    Announce,
    Query(String),
}

pub struct DiscoveryService {
    node_info: NodeInfo,
    socket: Option<Arc<UdpSocket>>,
    command_tx: Option<mpsc::Sender<DiscoveryCommand>>,
    discovered_nodes: Arc<Mutex<Vec<NodeInfo>>>,
    services: Vec<String>,
}

impl DiscoveryService {
    pub fn new(node_info: NodeInfo) -> Self {
        Self {
            node_info,
            socket: None,
            command_tx: None,
            discovered_nodes: Arc::new(Mutex::new(Vec::new())),
            services: vec!["storage".to_string(), "file-transfer".to_string()],
        }
    }
    
    pub async fn start(&mut self) -> Result<(), String> {
        // 创建 UDP 套接字
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", DISCOVERY_PORT))
            .await
            .map_err(|e| format!("Failed to bind socket: {}", e))?;
            
        // 加入多播组
        #[cfg(not(windows))]
        {
            use socket2::{Socket, Domain, Type};
            let addr = DISCOVERY_MULTICAST_ADDR.parse::<SocketAddr>()
                .map_err(|e| format!("Invalid multicast address: {}", e))?;
                
            let socket2 = Socket::new(Domain::IPV4, Type::DGRAM, None)
                .map_err(|e| format!("Failed to create socket: {}", e))?;
                
            socket2.join_multicast_v4(
                &addr.ip().to_string().parse::<std::net::Ipv4Addr>().unwrap(),
                &std::net::Ipv4Addr::new(0, 0, 0, 0)
            ).map_err(|e| format!("Failed to join multicast group: {}", e))?;
            
            // socket2 转换成 UdpSocket，这里简化了代码
        }
        
        let socket = Arc::new(socket);
        self.socket = Some(socket.clone());
        
        // 创建命令通道
        let (tx, rx) = mpsc::channel(10);
        self.command_tx = Some(tx);
        
        // 启动接收器
        let recv_socket = socket.clone();
        let discovered_nodes = self.discovered_nodes.clone();
        tokio::spawn(async move {
            let mut buf = [0; PACKET_SIZE];
            while let Ok((size, _src)) = recv_socket.recv_from(&mut buf).await {
                if let Ok(message_str) = std::str::from_utf8(&buf[..size]) {
                    if let Ok(message) = serde_json::from_str::<DiscoveryMessage>(message_str) {
                        match message {
                            DiscoveryMessage::Announce { node_id, name, host, port, .. } => {
                                // 处理节点宣告
                                let node_info = NodeInfo {
                                    id: node_id,
                                    name,
                                    host,
                                    port,
                                    status: 1, // NodeStatus::ONLINE
                                    last_seen: chrono::Utc::now().timestamp(),
                                };
                                
                                let mut nodes = discovered_nodes.lock().unwrap();
                                if !nodes.iter().any(|n| n.id == node_info.id) {
                                    nodes.push(node_info);
                                }
                            },
                            DiscoveryMessage::Query { .. } => {
                                // 处理查询请求
                                // 这里应该发送响应，但为简化代码，暂不实现
                            },
                            DiscoveryMessage::Response { .. } => {
                                // 处理响应
                                // 类似处理 Announce
                            }
                        }
                    }
                }
            }
        });
        
        // 启动命令处理器
        let cmd_socket = socket.clone();
        let node_info = self.node_info.clone();
        let services = self.services.clone();
        
        tokio::spawn(async move {
            let mut interval = time::interval(DISCOVERY_INTERVAL);
            let mut rx = rx;
            
            // 启动时发送初始公告
            Self::send_announce(&cmd_socket, &node_info, &services).await;
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // 定期发送公告
                        Self::send_announce(&cmd_socket, &node_info, &services).await;
                    }
                    Some(cmd) = rx.recv() => {
                        match cmd {
                            DiscoveryCommand::Stop => break,
                            DiscoveryCommand::Announce => {
                                Self::send_announce(&cmd_socket, &node_info, &services).await;
                            }
                            DiscoveryCommand::Query(network) => {
                                Self::send_query(&cmd_socket, &node_info.id, &network).await;
                            }
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
    
    pub async fn stop(&mut self) -> Result<(), String> {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(DiscoveryCommand::Stop).await;
            self.command_tx = None;
        }
        Ok(())
    }
    
    pub fn get_discovered_nodes(&self) -> Vec<NodeInfo> {
        let nodes = self.discovered_nodes.lock().unwrap();
        nodes.clone()
    }
    
    pub async fn trigger_announce(&self) -> Result<(), String> {
        if let Some(tx) = &self.command_tx {
            tx.send(DiscoveryCommand::Announce).await
                .map_err(|_| "Failed to send announce command".to_string())?;
            Ok(())
        } else {
            Err("Discovery service not started".to_string())
        }
    }
    
    pub async fn query_network(&self, network: &str) -> Result<(), String> {
        if let Some(tx) = &self.command_tx {
            tx.send(DiscoveryCommand::Query(network.to_string())).await
                .map_err(|_| "Failed to send query command".to_string())?;
            Ok(())
        } else {
            Err("Discovery service not started".to_string())
        }
    }
    
    async fn send_announce(socket: &UdpSocket, node_info: &NodeInfo, services: &[String]) {
        let message = DiscoveryMessage::Announce {
            node_id: node_info.id.clone(),
            name: node_info.name.clone(),
            host: node_info.host.clone(),
            port: node_info.port,
            services: services.to_vec(),
        };
        
        if let Ok(message_json) = serde_json::to_string(&message) {
            if let Ok(addr) = DISCOVERY_MULTICAST_ADDR.parse::<SocketAddr>() {
                let _ = socket.send_to(message_json.as_bytes(), addr).await;
            }
        }
    }
    
    async fn send_query(socket: &UdpSocket, node_id: &str, network: &str) {
        let message = DiscoveryMessage::Query {
            node_id: node_id.to_string(),
            network: network.to_string(),
        };
        
        if let Ok(message_json) = serde_json::to_string(&message) {
            if let Ok(addr) = DISCOVERY_MULTICAST_ADDR.parse::<SocketAddr>() {
                let _ = socket.send_to(message_json.as_bytes(), addr).await;
            }
        }
    }
} 