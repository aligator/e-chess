pub struct LichessConnector<R: Requester> {
    request: R,
    upstream_rx: Receiver<String>,
    upstream_tx: Sender<String>,
}

impl<R: Requester> LichessConnector<R> {
    pub fn new(request: R) -> Self {
        let (tx, rx) = mpsc::channel();

        Self {
            request,
            upstream_rx: rx,
            upstream_tx: tx,
        }
    }
}
