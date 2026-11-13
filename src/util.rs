//! Utilitários internos compartilhados entre os módulos.

use std::io::{BufRead, BufReader, Read};
use std::sync::mpsc::Sender;

use crate::cleaner::Msg;

/// Consome um stream de texto linha a linha, encaminhando cada linha para o
/// canal de progresso. Roda em uma thread dedicada para não bloquear o
/// processo pai.
///
/// Usado tanto na limpeza (`cleaner`) quanto no desinstalador (`tools`).
pub fn drenar<S: Read + Send + 'static>(stream: Option<S>, tx: &Sender<Msg>) {
    if let Some(stream) = stream {
        let tx2 = tx.clone();
        std::thread::spawn(move || {
            for linha in BufReader::new(stream).lines().map_while(Result::ok) {
                let _ = tx2.send(Msg::Linha(linha));
            }
        });
    }
}
