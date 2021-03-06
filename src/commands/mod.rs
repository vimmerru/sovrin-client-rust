pub mod anoncreds;
pub mod ledger;
pub mod pool;
pub mod signus;
pub mod wallet;

use commands::anoncreds::{AnoncredsCommand, AnoncredsCommandExecutor};
use commands::ledger::{LedgerCommand, LedgerCommandExecutor};
use commands::pool::{PoolCommand, PoolCommandExecutor};
use commands::signus::{SignusCommand, SignusCommandExecutor};
use commands::wallet::{WalletCommand, WalletCommandExecutor};

use services::crypto::CryptoService;
use services::pool::PoolService;
use services::wallet::WalletService;

use std::error;
use std::sync::mpsc::{Sender, channel};
use std::rc::Rc;
use std::thread;
use std::sync::{Mutex, MutexGuard};

pub enum Command {
    Exit,
    Anoncreds(AnoncredsCommand),
    Ledger(LedgerCommand),
    Pool(PoolCommand),
    Signus(SignusCommand),
    Wallet(WalletCommand)
}

pub struct CommandExecutor {
    worker: Option<thread::JoinHandle<()>>,
    sender: Sender<Command>
}

/// Global (lazy inited) instance of CommandExecutor
///
/// Sample:
///
/// {
///     ...
///     let ref ce: CommandExecutor = *CommandExecutor::instance();                <- lock +
///     ce.send(Command::Exit);                                                            |
///     ...                                                                                |
/// }                                                                            <- unlock +
lazy_static! {
    static ref COMMAND_EXECUTOR: Mutex<CommandExecutor> = Mutex::new(CommandExecutor::new());
}

impl CommandExecutor {
    pub fn instance<'mutex>() -> MutexGuard<'mutex, CommandExecutor> {
        COMMAND_EXECUTOR.lock().unwrap()
    }

    fn new() -> CommandExecutor {
        let (sender, receiver) = channel();

        CommandExecutor {
            sender: sender,
            worker: Some(thread::spawn(move || {
                info!(target: "command_executor", "Worker thread started");

                let crypto_service = Rc::new(CryptoService::new());
                let pool_service = Rc::new(PoolService::new());
                let wallet_service = Rc::new(WalletService::new());

                let anoncreds_command_executor = AnoncredsCommandExecutor::new(crypto_service.clone(), pool_service.clone(), wallet_service.clone());
                let ledger_command_executor = LedgerCommandExecutor::new(crypto_service.clone(), pool_service.clone(), wallet_service.clone());
                let pool_command_executor = PoolCommandExecutor::new(pool_service.clone());
                let signus_command_executor = SignusCommandExecutor::new(crypto_service.clone(), pool_service.clone(), wallet_service.clone());
                let wallet_command_executor = WalletCommandExecutor::new(pool_service.clone(), wallet_service.clone());

                loop {
                    match receiver.recv() {
                        Ok(Command::Anoncreds(cmd)) => {
                            info!(target: "command_executor", "AnoncredsCommand command received");
                            anoncreds_command_executor.execute(cmd);
                        },
                        Ok(Command::Ledger(cmd)) => {
                            info!(target: "command_executor", "LedgerCommand command received");
                            ledger_command_executor.execute(cmd);
                        },
                        Ok(Command::Pool(cmd)) => {
                            info!(target: "command_executor", "PoolCommand command received");
                            pool_command_executor.execute(cmd);
                        },
                        Ok(Command::Signus(cmd)) => {
                            info!(target: "command_executor", "SignusCommand command received");
                            signus_command_executor.execute(cmd);
                        },
                        Ok(Command::Wallet(cmd)) => {
                            info!(target: "command_executor", "WalletCommand command received");
                            wallet_command_executor.execute(cmd);
                        },
                        Ok(Command::Exit) => {
                            info!(target: "command_executor", "Exit command received");
                            break
                        },
                        Err(err) => {
                            error!(target: "command_executor", "Failed to get command!");
                            panic!("Failed to get command! {:?}", err)
                        }
                    }
                }
            }))
        }
    }

    pub fn send(&self, cmd: Command) {
        self.sender.send(cmd);
    }
}

impl Drop for CommandExecutor {
    fn drop(&mut self) {
        info!(target: "command_executor", "Drop started");
        self.send(Command::Exit);
        // Option worker type and this kludge is workaround for rust
        self.worker.take().unwrap().join();
        info!(target: "command_executor", "Drop finished");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_executor_can_be_created() {
        let command_executor = CommandExecutor::new();
        assert!(true, "No crashes on CommandExecutor::new");
    }

    #[test]
    fn command_executor_can_be_dropped() {
        fn drop_test() {
            let command_executor = CommandExecutor::new();
        }

        drop_test();
        assert!(true, "No crashes on CommandExecutor::drop");
    }

    #[test]
    fn command_executor_can_get_instance() {
        let ref command_executor: CommandExecutor = *CommandExecutor::instance();
        // Deadlock if another one instance will be requested (try to uncomment the next line)
        // let ref other_ce: CommandExecutor = *CommandExecutor::instance();
    }
}