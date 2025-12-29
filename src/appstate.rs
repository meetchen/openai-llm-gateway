

use crate::inference::worker::InferenceCommand;

#[derive(Clone, Debug)]
pub struct AppState {

    pub worker_tx: tokio::sync::mpsc::Sender<InferenceCommand>,
}
