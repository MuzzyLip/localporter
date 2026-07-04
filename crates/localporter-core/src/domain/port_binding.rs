#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PortProtocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundPort {
    pub protocol: PortProtocol,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessPortBinding {
    pub pid: u32,
    pub process_name: String,
    pub port: BoundPort,
}
