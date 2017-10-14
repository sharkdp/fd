use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;

use super::TokenizedCommand;

/// An event loop that listens for inputs from the `rx` receiver. Each received input will
/// generate a command with the supplied command template. The generated command will then
/// be executed, and this process will continue until the receiver's sender has closed.
pub fn job(
    rx: Arc<Mutex<Receiver<PathBuf>>>,
    base: Arc<Option<PathBuf>>,
    cmd: Arc<TokenizedCommand>,
) {
    // A string buffer that will be re-used in each iteration.
    let buffer = &mut String::with_capacity(256);

    loop {
        // Create a lock on the shared receiver for this thread.
        let lock = rx.lock().unwrap();

        // Obtain the next path from the receiver, else if the channel
        // has closed, exit from the loop
        let value: PathBuf = match lock.recv() {
            Ok(value) => {
                match *base {
                    Some(ref base) => base.join(&value),
                    None => value,
                }
            }
            Err(_) => break,
        };

        // Drop the lock so that other threads can read from the the receiver.
        drop(lock);
        // Generate a command to store within the buffer, and execute the command.
        // Note that the `then_execute()` method will clear the buffer for us.
        cmd.generate(buffer, &value).then_execute();
    }
}
