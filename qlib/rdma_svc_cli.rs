use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::ops::Deref;
use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering;
use spin::Mutex;

use super::common::*;
use super::fileinfo::*;
use super::kernel::tcpip::tcpip::*;
use super::kernel::GlobalIOMgr;
use super::kernel::GlobalRDMASvcCli;
use super::linux_def::*;
use super::rdma_share::*;
use super::rdmasocket::*;
use super::socket_buf::*;
use super::unix_socket::UnixSocket;

pub struct RDMASvcCliIntern {
    // agent id
    pub agentId: u32,

    // the unix socket fd between rdma client and RDMASrv
    pub cliSock: UnixSocket,

    // the memfd share memory with rdma client
    pub cliMemFd: i32,

    // the memfd share memory with rdma server
    pub srvMemFd: i32,

    // the eventfd which send notification to client
    pub cliEventFd: i32,

    // the eventfd which send notification to client
    pub srvEventFd: i32,

    // the memory region shared with client
    pub cliMemRegion: MemRegion,

    pub cliShareRegion: Mutex<&'static mut ClientShareRegion>,

    // srv memory region shared with all RDMAClient
    pub srvMemRegion: MemRegion,

    // the bitmap to expedite ready container search
    pub srvShareRegion: Mutex<&'static mut ShareRegion>,

    //TODO: rename, it's the channelId to RDMAId's mapping
    pub channelToSocketMappings: Mutex<BTreeMap<u32, u32>>,

    pub rdmaIdToSocketMappings: Mutex<BTreeMap<u32, i32>>,

    pub nextRDMAId: AtomicU32,

    pub podId: [u8; 64],
}

impl Deref for RDMASvcClient {
    type Target = Arc<RDMASvcCliIntern>;

    fn deref(&self) -> &Arc<RDMASvcCliIntern> {
        &self.intern
    }
}

pub struct RDMASvcClient {
    pub intern: Arc<RDMASvcCliIntern>,
}

impl Default for RDMASvcClient {
    fn default() -> Self {
        Self {
            intern: Arc::new(RDMASvcCliIntern {
                agentId: 0,
                cliSock: UnixSocket { fd: -1 },
                cliMemFd: 0,
                srvMemFd: 0,
                srvEventFd: 0,
                cliEventFd: 0,
                cliMemRegion: MemRegion { addr: 0, len: 0 },
                cliShareRegion: unsafe { Mutex::new(&mut (*(0 as *mut ClientShareRegion))) },
                srvMemRegion: MemRegion { addr: 0, len: 0 },
                srvShareRegion: unsafe { Mutex::new(&mut (*(0 as *mut ShareRegion))) },
                channelToSocketMappings: Mutex::new(BTreeMap::new()),
                rdmaIdToSocketMappings: Mutex::new(BTreeMap::new()),
                nextRDMAId: AtomicU32::new(0), //AtomicU64::new((i32::MAX + 1) as u64), //2147483647 + 1 = 2147483648
                podId: [0; 64],
            }),
        }
    }
}

impl RDMASvcClient {
    pub fn listen(&self, sockfd: u32, endpoint: &Endpoint, waitingLen: i32) -> Result<()> {
        let res = self.SentMsgToSvc(RDMAReqMsg::RDMAListen(RDMAListenReq {
            sockfd: sockfd,
            ipAddr: endpoint.ipAddr,
            port: endpoint.port,
            waitingLen,
        }));
        res
    }

    pub fn listenUsingPodId(&self, sockfd: u32, port: u16, waitingLen: i32) -> Result<()> {
        let res = self.SentMsgToSvc(RDMAReqMsg::RDMAListenUsingPodId(RDMAListenReqUsingPodId {
            sockfd: sockfd,
            podId: self.podId,
            port,
            waitingLen,
        }));
        res
    }

    pub fn connect(
        &self,
        sockfd: u32,
        dstIpAddr: u32,
        dstPort: u16,
        srcIpAddr: u32,
        srcPort: u16,
    ) -> Result<()> {
        let res = self.SentMsgToSvc(RDMAReqMsg::RDMAConnect(RDMAConnectReq {
            sockfd,
            dstIpAddr,
            dstPort,
            srcIpAddr, //101099712, //u32::from(Ipv4Addr::from_str("192.168.6.6").unwrap()).to_be(),
            srcPort,   //16866u16.to_be(),
        }));
        res
    }

    pub fn connectUsingPodId(
        &self,
        sockfd: u32,
        dstIpAddr: u32,
        dstPort: u16,
        srcPort: u16,
    ) -> Result<()> {
        let res = self.SentMsgToSvc(RDMAReqMsg::RDMAConnectUsingPodId(
            RDMAConnectReqUsingPodId {
                sockfd,
                dstIpAddr,
                dstPort,
                podId: self.podId, //101099712, //u32::from(Ipv4Addr::from_str("192.168.6.6").unwrap()).to_be(),
                srcPort,           //16866u16.to_be(),
            },
        ));
        res
    }

    pub fn read(&self, channelId: u32) -> Result<()> {
        // println!("rdmaSvcCli::read 1");
        if self.cliShareRegion.lock().sq.Push(RDMAReq {
            user_data: 0,
            msg: RDMAReqMsg::RDMARead(RDMAReadReq {
                channelId: channelId,
            }),
        }) {
            // println!("rdmaSvcCli::read 2");
            self.updateBitmapAndWakeUpServerIfNecessary();
            Ok(())
        } else {
            // println!("rdmaSvcCli::read 3");
            return Err(Error::NoEnoughSpace);
        }
    }

    pub fn write(&self, channelId: u32) -> Result<()> {
        // println!("rdmaSvcCli::write 1");
        if self.cliShareRegion.lock().sq.Push(RDMAReq {
            user_data: 0,
            msg: RDMAReqMsg::RDMAWrite(RDMAWriteReq {
                channelId: channelId,
            }),
        }) {
            // println!("rdmaSvcCli::write 2");
            self.updateBitmapAndWakeUpServerIfNecessary();
            Ok(())
        } else {
            // println!("rdmaSvcCli::write 3");
            return Err(Error::NoEnoughSpace);
        }
    }

    pub fn shutdown(&self, channelId: u32, howto: u8) -> Result<()> {
        // println!(
        //     "rdmaSvcCli::shutdown 1, channelId: {}, howto: {}",
        //     channelId, howto
        // );
        if self.cliShareRegion.lock().sq.Push(RDMAReq {
            user_data: 0,
            msg: RDMAReqMsg::RDMAShutdown(RDMAShutdownReq {
                channelId: channelId,
                howto,
            }),
        }) {
            // println!("rdmaSvcCli::shutdown 2");
            self.updateBitmapAndWakeUpServerIfNecessary();
            Ok(())
        } else {
            // println!("rdmaSvcCli::shutdown 3");
            return Err(Error::NoEnoughSpace);
        }
    }

    pub fn close(&self, channelId: u32) -> Result<()> {
        let res = self.SentMsgToSvc(RDMAReqMsg::RDMAClose(RDMACloseReq { channelId }));
        res
    }

    pub fn updateBitmapAndWakeUpServerIfNecessary(&self) {
        // println!("updateBitmapAndWakeUpServerIfNecessary 1 ");
        let mut srvShareRegion = self.srvShareRegion.lock();
        // println!("updateBitmapAndWakeUpServerIfNecessary 2 ");
        srvShareRegion.updateBitmap(self.agentId);
        if srvShareRegion.srvBitmap.load(Ordering::Acquire) == 1 {
            self.wakeupSvc();
        } else {
            // println!("server is not sleeping");
            // self.updateBitmapAndWakeUpServerIfNecessary();
        }
    }

    pub fn SentMsgToSvc(&self, msg: RDMAReqMsg) -> Result<()> {
        if self
            .cliShareRegion
            .lock()
            .sq
            .Push(RDMAReq { user_data: 0, msg })
        {
            self.updateBitmapAndWakeUpServerIfNecessary();
            Ok(())
        } else {
            return Err(Error::NoEnoughSpace);
        }
    }

    pub fn DrainCompletionQueue(&self) -> usize {
        self.cliShareRegion
            .lock()
            .clientBitmap
            .store(0, Ordering::Release);
        let mut count = 0;
        count += self.ProcessRDMASvcMessage();
        self.cliShareRegion
            .lock()
            .clientBitmap
            .store(1, Ordering::Release);
        count += self.ProcessRDMASvcMessage();
        count
    }

    pub fn ProcessRDMASvcMessage(&self) -> usize {
        let mut count = 0;
        loop {
            let request = self.cliShareRegion.lock().cq.Pop();
            count += 1;
            match request {
                Some(cq) => match cq.msg {
                    RDMARespMsg::RDMAConnect(response) => {
                        // debug!("RDMARespMsg::RDMAConnect, response: {:?}", response);
                        let sockfd: i32;
                        let rdmaIdToSocketMappings =
                            GlobalRDMASvcCli().rdmaIdToSocketMappings.lock();
                        let sockfdOption = rdmaIdToSocketMappings.get(&response.sockfd);
                        match sockfdOption {
                            Some(sockfdVal) => {
                                sockfd = *sockfdVal;
                            }
                            None => {
                                debug!("Can't find sockfd from rdmaId: {}", response.sockfd);
                                break;
                            }
                        }
                        let fdInfo = GlobalIOMgr().GetByHost(sockfd).unwrap();
                        let fdInfoLock = fdInfo.lock();
                        let sockInfo = fdInfoLock.sockInfo.lock().clone();

                        match sockInfo {
                            SockInfo::Socket(_) => {
                                let ioBufIndex = response.ioBufIndex as usize;
                                let shareRegion = self.cliShareRegion.lock();
                                let sockBuf = SocketBuff(Arc::new(SocketBuffIntern::InitWithShareMemory(
                                    MemoryDef::DEFAULT_BUF_PAGE_COUNT,
                                    &shareRegion.ioMetas[ioBufIndex].readBufAtoms as *const _
                                        as u64,
                                    &shareRegion.ioMetas[ioBufIndex].writeBufAtoms as *const _
                                        as u64,
                                    &shareRegion.ioMetas[ioBufIndex].consumeReadData as *const _
                                        as u64,
                                    &shareRegion.iobufs[ioBufIndex].read as *const _ as u64,
                                    &shareRegion.iobufs[ioBufIndex].write as *const _ as u64,
                                    false,
                                )));

                                let dataSock = RDMADataSock::New(
                                    response.sockfd,
                                    sockBuf.clone(),
                                    response.channelId,
                                    response.srcIpAddr,
                                    response.srcPort,
                                    response.dstIpAddr,
                                    response.dstPort,
                                );
                                self.channelToSocketMappings
                                    .lock()
                                    .insert(response.channelId, response.sockfd);

                                *fdInfoLock.sockInfo.lock() = SockInfo::RDMADataSocket(dataSock);
                                fdInfoLock.waitInfo.Notify(EVENT_OUT);
                            }
                            _ => {
                                panic!("SockInfo is not correct type");
                            }
                        }
                    }
                    RDMARespMsg::RDMAAccept(response) => {
                        // debug!("RDMARespMsg::RDMAAccept, response: {:?}", response);
                        let sockfd;
                        let mut rdmaIdToSocketMappings =
                            GlobalRDMASvcCli().rdmaIdToSocketMappings.lock();
                        let sockfdOption = rdmaIdToSocketMappings.get(&response.sockfd);
                        match sockfdOption {
                            Some(sockfdVal) => {
                                sockfd = *sockfdVal;
                            }
                            None => {
                                debug!("Can't find sockfd from rdmaId: {}", response.sockfd);
                                break;
                            }
                        }
                        let fdInfo = GlobalIOMgr().GetByHost(sockfd).unwrap();
                        let fdInfoLock = fdInfo.lock();
                        let sockInfo = fdInfoLock.sockInfo.lock().clone();

                        match sockInfo {
                            SockInfo::RDMAServerSocket(rdmaServerSock) => {
                                // let fd = unsafe { libc::socket(AFType::AF_INET, SOCK_STREAM, 0) };
                                let fd = self.CreateSocket() as i32;
                                let rdmaId = GlobalRDMASvcCli()
                                    .nextRDMAId
                                    .fetch_add(1, Ordering::Release);
                                rdmaIdToSocketMappings.insert(rdmaId, fd);
                                let ioBufIndex = response.ioBufIndex as usize;
                                let shareRegion = self.cliShareRegion.lock();
                                let sockBuf = SocketBuff(Arc::new(SocketBuffIntern::InitWithShareMemory(
                                    MemoryDef::DEFAULT_BUF_PAGE_COUNT,
                                    &shareRegion.ioMetas[ioBufIndex].readBufAtoms as *const _
                                        as u64,
                                    &shareRegion.ioMetas[ioBufIndex].writeBufAtoms as *const _
                                        as u64,
                                    &shareRegion.ioMetas[ioBufIndex].consumeReadData as *const _
                                        as u64,
                                    &shareRegion.iobufs[ioBufIndex].read as *const _ as u64,
                                    &shareRegion.iobufs[ioBufIndex].write as *const _ as u64,
                                    false,
                                )));

                                let dataSock = RDMADataSock::New(
                                    rdmaId,
                                    sockBuf.clone(),
                                    response.channelId,
                                    response.srcIpAddr,
                                    response.srcPort,
                                    response.dstIpAddr,
                                    response.dstPort,
                                );

                                let fdInfo = GlobalIOMgr().GetByHost(fd as i32).unwrap();
                                let fdInfoLock1 = fdInfo.lock();
                                *fdInfoLock1.sockInfo.lock() = SockInfo::RDMADataSocket(dataSock);

                                let sockAddr = SockAddr::Inet(SockAddrInet {
                                    Family: AFType::AF_INET as u16,
                                    Port: response.dstPort,
                                    Addr: response.dstIpAddr.to_be_bytes(),
                                    Zero: [0; 8],
                                });
                                let mut tcpSockAddr = TcpSockAddr::default();
                                let len = sockAddr.Len();
                                let _res = sockAddr.Marsh(&mut tcpSockAddr.data, len);
                                let (trigger, _tmp) = rdmaServerSock.acceptQueue.lock().EnqSocket(
                                    fd,
                                    tcpSockAddr,
                                    len as u32,
                                    sockBuf,
                                );
                                self.channelToSocketMappings
                                    .lock()
                                    .insert(response.channelId, rdmaId);
                                if trigger {
                                    fdInfoLock.waitInfo.Notify(EVENT_IN);
                                }
                            }
                            _ => {
                                panic!("SockInfo is not correct type");
                            }
                        }
                    }
                    RDMARespMsg::RDMANotify(response) => {
                        // debug!("RDMARespMsg::RDMANotify, response: {:?}", response);
                        let mut channelToSocketMappings = self.channelToSocketMappings.lock();
                        let rdmaId;
                        let rdmaIdOption = channelToSocketMappings.get_mut(&response.channelId);
                        match rdmaIdOption {
                            Some(rdmaIdVal) => {
                                rdmaId = rdmaIdVal;
                            }
                            None => {
                                debug!(
                                    "Can't find rdmaId based on channelId: {}",
                                    response.channelId
                                );
                                break;
                            }
                        }

                        let sockFd: i32;
                        let rdmaIdToSocketMappings = self.rdmaIdToSocketMappings.lock();
                        let sockFdOption = rdmaIdToSocketMappings.get(rdmaId);
                        match sockFdOption {
                            Some(sockFdVal) => {
                                sockFd = *sockFdVal;
                            }
                            None => {
                                debug!("Can't find sockfd based on rdmaId: {}", rdmaId);
                                break;
                            }
                        }
                        let fdInfo = GlobalIOMgr().GetByHost(sockFd).unwrap();
                        let fdInfo = fdInfo.lock();
                        if response.event & EVENT_IN != 0 {
                            fdInfo.waitInfo.Notify(EVENT_IN);
                        }
                        if response.event & EVENT_OUT != 0 {
                            fdInfo.waitInfo.Notify(EVENT_OUT);
                        }
                    }
                    RDMARespMsg::RDMAFinNotify(response) => {
                        debug!("RDMARespMsg::RDMAFinNotify, response: {:?}", response);
                        let mut channelToSocketMappings = self.channelToSocketMappings.lock();
                        let rdmaId;
                        let rdmaIdOption = channelToSocketMappings.get_mut(&response.channelId);
                        match rdmaIdOption {
                            Some(rdmaIdVal) => {
                                rdmaId = rdmaIdVal;
                            }
                            None => {
                                debug!(
                                    "Can't find rdmaId based on channelId: {}",
                                    response.channelId
                                );
                                break;
                            }
                        }

                        let sockFd;
                        let rdmaIdToSocketMappings = self.rdmaIdToSocketMappings.lock();
                        let sockFdOption = rdmaIdToSocketMappings.get(rdmaId);
                        match sockFdOption {
                            Some(sockFdVal) => {
                                sockFd = *sockFdVal;
                            }
                            None => {
                                debug!("Can't find sockfd based on rdmaId: {}", rdmaId);
                                break;
                            }
                        }
                        let fdInfo = GlobalIOMgr().GetByHost(sockFd).unwrap();
                        let fdInfo = fdInfo.lock();
                        let sockInfo = fdInfo.sockInfo.lock().clone();
                        match sockInfo {
                            SockInfo::RDMADataSocket(dataSock) => {
                                dataSock.socketBuf.SetRClosed();
                                fdInfo.waitInfo.Notify(EVENT_IN);
                            }
                            _ => {
                                debug!("Unexpected sockInfo type: {:?}", sockInfo);
                            }
                        }
                    }
                },
                None => {
                    count -= 1;
                    break;
                }
            }
        }
        count
    }
}
