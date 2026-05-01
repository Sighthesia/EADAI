/// Handshake state machine for the self-describing protocol.
///
/// This module manages the state transitions during the handshake phase
/// between device and host.
use super::frame::*;

/// Handshake states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HandshakeState {
    /// Waiting for device identity.
    WaitingIdentity,
    /// Identity received, waiting for command catalog.
    WaitingCommandCatalog,
    /// Command catalog complete, waiting for variable catalog.
    WaitingVariableCatalog,
    /// Variable catalog complete, waiting for host acknowledgment.
    WaitingHostAck,
    /// Host acknowledged, ready to start streaming.
    ReadyToStream,
    /// Streaming is active.
    Streaming,
    /// Error occurred during handshake.
    Error(String),
}

/// Handshake state machine.
pub struct HandshakeMachine {
    /// Current state.
    state: HandshakeState,
    /// Device identity (if received).
    identity: Option<Identity>,
    /// Command catalog pages received.
    command_catalog_pages: Vec<CommandCatalogPage>,
    /// Variable catalog pages received.
    variable_catalog_pages: Vec<VariableCatalogPage>,
    /// Expected total command catalog pages.
    expected_command_pages: Option<u16>,
    /// Expected total variable catalog pages.
    expected_variable_pages: Option<u16>,
}

impl HandshakeMachine {
    /// Create a new handshake machine in the initial state.
    pub fn new() -> Self {
        Self {
            state: HandshakeState::WaitingIdentity,
            identity: None,
            command_catalog_pages: Vec::new(),
            variable_catalog_pages: Vec::new(),
            expected_command_pages: None,
            expected_variable_pages: None,
        }
    }

    /// Get the current state.
    pub fn state(&self) -> &HandshakeState {
        &self.state
    }

    /// Get the received identity (if any).
    pub fn identity(&self) -> Option<&Identity> {
        self.identity.as_ref()
    }

    /// Process an identity frame.
    pub fn on_identity(&mut self, identity: Identity) -> Result<(), String> {
        if self.state != HandshakeState::WaitingIdentity {
            return Err(format!("unexpected identity in state {:?}", self.state));
        }

        self.identity = Some(identity);
        self.state = HandshakeState::WaitingCommandCatalog;
        Ok(())
    }

    /// Process a command catalog page.
    pub fn on_command_catalog_page(&mut self, page: CommandCatalogPage) -> Result<(), String> {
        if self.state != HandshakeState::WaitingCommandCatalog {
            return Err(format!(
                "unexpected command catalog page in state {:?}",
                self.state
            ));
        }

        // Validate page number
        if self.expected_command_pages.is_none() {
            self.expected_command_pages = Some(page.total_pages);
        } else if self.expected_command_pages != Some(page.total_pages) {
            return Err("command catalog total_pages mismatch".to_string());
        }

        // Check for duplicate pages
        if self
            .command_catalog_pages
            .iter()
            .any(|p| p.page == page.page)
        {
            return Err(format!("duplicate command catalog page {}", page.page));
        }

        self.command_catalog_pages.push(page);

        // Check if we have all pages
        if let Some(total) = self.expected_command_pages
            && self.command_catalog_pages.len() == total as usize
        {
            self.state = HandshakeState::WaitingVariableCatalog;
        }

        Ok(())
    }

    /// Process a variable catalog page.
    pub fn on_variable_catalog_page(&mut self, page: VariableCatalogPage) -> Result<(), String> {
        if self.state != HandshakeState::WaitingVariableCatalog {
            return Err(format!(
                "unexpected variable catalog page in state {:?}",
                self.state
            ));
        }

        // Validate page number
        if self.expected_variable_pages.is_none() {
            self.expected_variable_pages = Some(page.total_pages);
        } else if self.expected_variable_pages != Some(page.total_pages) {
            return Err("variable catalog total_pages mismatch".to_string());
        }

        // Check for duplicate pages
        if self
            .variable_catalog_pages
            .iter()
            .any(|p| p.page == page.page)
        {
            return Err(format!("duplicate variable catalog page {}", page.page));
        }

        self.variable_catalog_pages.push(page);

        // Check if we have all pages
        if let Some(total) = self.expected_variable_pages
            && self.variable_catalog_pages.len() == total as usize
        {
            self.state = HandshakeState::WaitingHostAck;
        }

        Ok(())
    }

    /// Process a host acknowledgment.
    ///
    /// Stage-level ACKs are accepted in the state *after* the corresponding data
    /// has been received, because the host sends the ack to confirm receipt:
    ///
    /// - `Identity` ack → valid once identity is received (`WaitingCommandCatalog`)
    /// - `CommandCatalog` ack → valid once all command pages are received (`WaitingVariableCatalog`)
    /// - `VariableCatalog` ack → valid once all variable pages are received (`WaitingHostAck`)
    /// - `Streaming` ack → valid once streaming is approved (`ReadyToStream`)
    pub fn on_host_ack(&mut self, ack: &HostAck) -> Result<(), String> {
        if ack.status != 0 {
            return Err(format!("host ack error: {}", ack.message));
        }

        match ack.stage {
            AckStage::Identity => {
                if self.state != HandshakeState::WaitingCommandCatalog {
                    return Err(format!("unexpected identity ack in state {:?}", self.state));
                }
                // Identity ack confirms receipt; state already advanced past identity.
            }
            AckStage::CommandCatalog => {
                if self.state != HandshakeState::WaitingVariableCatalog {
                    return Err(format!(
                        "unexpected command catalog ack in state {:?}",
                        self.state
                    ));
                }
                // Command catalog ack confirms receipt; state already advanced past command catalog.
            }
            AckStage::VariableCatalog => {
                if self.state != HandshakeState::WaitingHostAck {
                    return Err(format!(
                        "unexpected variable catalog ack in state {:?}",
                        self.state
                    ));
                }
                self.state = HandshakeState::ReadyToStream;
            }
            AckStage::Streaming => {
                if self.state != HandshakeState::ReadyToStream {
                    return Err(format!(
                        "unexpected streaming ack in state {:?}",
                        self.state
                    ));
                }
                self.state = HandshakeState::Streaming;
            }
        }

        Ok(())
    }

    /// Start streaming (called after ReadyToStream).
    pub fn start_streaming(&mut self) -> Result<(), String> {
        if self.state != HandshakeState::ReadyToStream {
            return Err(format!("cannot start streaming in state {:?}", self.state));
        }
        self.state = HandshakeState::Streaming;
        Ok(())
    }

    /// Get all command catalog entries (sorted by page order).
    pub fn command_catalog(&self) -> Vec<CommandDescriptor> {
        let mut pages = self.command_catalog_pages.clone();
        pages.sort_by_key(|p| p.page);
        pages
            .into_iter()
            .flat_map(|p| p.commands.into_iter())
            .collect()
    }

    /// Get all variable catalog entries (sorted by page order, then by order field).
    pub fn variable_catalog(&self) -> Vec<VariableDescriptor> {
        let mut pages = self.variable_catalog_pages.clone();
        pages.sort_by_key(|p| p.page);
        let mut vars: Vec<VariableDescriptor> = pages
            .into_iter()
            .flat_map(|p| p.variables.into_iter())
            .collect();
        vars.sort_by_key(|v| v.order);
        vars
    }
}

impl Default for HandshakeMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_identity() -> Identity {
        Identity {
            protocol_version: 1,
            device_name: "Test Device".to_string(),
            firmware_version: "1.0.0".to_string(),
            sample_rate_hz: 100,
            variable_count: 2,
            command_count: 1,
            sample_payload_len: 8,
        }
    }

    fn sample_command_catalog_page() -> CommandCatalogPage {
        CommandCatalogPage {
            page: 0,
            total_pages: 1,
            commands: vec![CommandDescriptor {
                id: "start".to_string(),
                params: "".to_string(),
                docs: "Start streaming".to_string(),
            }],
        }
    }

    fn sample_variable_catalog_page() -> VariableCatalogPage {
        VariableCatalogPage {
            page: 0,
            total_pages: 1,
            variables: vec![
                VariableDescriptor {
                    name: "acc_x".to_string(),
                    order: 0,
                    unit: "m/s^2".to_string(),
                    adjustable: false,
                    value_type: ValueType::I16,
                },
                VariableDescriptor {
                    name: "gain".to_string(),
                    order: 1,
                    unit: "".to_string(),
                    adjustable: true,
                    value_type: ValueType::F32,
                },
            ],
        }
    }

    #[test]
    fn test_handshake_flow() {
        let mut machine = HandshakeMachine::new();
        assert_eq!(*machine.state(), HandshakeState::WaitingIdentity);

        // Receive identity
        machine.on_identity(sample_identity()).unwrap();
        assert_eq!(*machine.state(), HandshakeState::WaitingCommandCatalog);

        // Receive command catalog
        machine
            .on_command_catalog_page(sample_command_catalog_page())
            .unwrap();
        assert_eq!(*machine.state(), HandshakeState::WaitingVariableCatalog);

        // Receive variable catalog
        machine
            .on_variable_catalog_page(sample_variable_catalog_page())
            .unwrap();
        assert_eq!(*machine.state(), HandshakeState::WaitingHostAck);

        // Host acknowledges
        machine
            .on_host_ack(&HostAck {
                stage: AckStage::VariableCatalog,
                status: 0,
                message: "OK".to_string(),
            })
            .unwrap();
        assert_eq!(*machine.state(), HandshakeState::ReadyToStream);

        // Start streaming
        machine.start_streaming().unwrap();
        assert_eq!(*machine.state(), HandshakeState::Streaming);
    }

    #[test]
    fn test_multi_page_catalog() {
        let mut machine = HandshakeMachine::new();
        machine.on_identity(sample_identity()).unwrap();

        // Two command catalog pages
        machine
            .on_command_catalog_page(CommandCatalogPage {
                page: 0,
                total_pages: 2,
                commands: vec![CommandDescriptor {
                    id: "start".to_string(),
                    params: "".to_string(),
                    docs: "Start".to_string(),
                }],
            })
            .unwrap();
        assert_eq!(*machine.state(), HandshakeState::WaitingCommandCatalog);

        machine
            .on_command_catalog_page(CommandCatalogPage {
                page: 1,
                total_pages: 2,
                commands: vec![CommandDescriptor {
                    id: "stop".to_string(),
                    params: "".to_string(),
                    docs: "Stop".to_string(),
                }],
            })
            .unwrap();
        assert_eq!(*machine.state(), HandshakeState::WaitingVariableCatalog);

        // Two variable catalog pages
        machine
            .on_variable_catalog_page(VariableCatalogPage {
                page: 0,
                total_pages: 2,
                variables: vec![VariableDescriptor {
                    name: "acc_x".to_string(),
                    order: 0,
                    unit: "m/s^2".to_string(),
                    adjustable: false,
                    value_type: ValueType::I16,
                }],
            })
            .unwrap();
        assert_eq!(*machine.state(), HandshakeState::WaitingVariableCatalog);

        machine
            .on_variable_catalog_page(VariableCatalogPage {
                page: 1,
                total_pages: 2,
                variables: vec![VariableDescriptor {
                    name: "gain".to_string(),
                    order: 1,
                    unit: "".to_string(),
                    adjustable: true,
                    value_type: ValueType::F32,
                }],
            })
            .unwrap();
        assert_eq!(*machine.state(), HandshakeState::WaitingHostAck);

        // Check catalogs are merged correctly
        let cmd_catalog = machine.command_catalog();
        assert_eq!(cmd_catalog.len(), 2);
        assert_eq!(cmd_catalog[0].id, "start");
        assert_eq!(cmd_catalog[1].id, "stop");

        let var_catalog = machine.variable_catalog();
        assert_eq!(var_catalog.len(), 2);
        assert_eq!(var_catalog[0].name, "acc_x");
        assert_eq!(var_catalog[1].name, "gain");
    }

    #[test]
    fn test_duplicate_page_error() {
        let mut machine = HandshakeMachine::new();
        machine.on_identity(sample_identity()).unwrap();

        machine
            .on_command_catalog_page(sample_command_catalog_page())
            .unwrap();
        assert!(
            machine
                .on_command_catalog_page(sample_command_catalog_page())
                .is_err()
        );
    }

    #[test]
    fn test_wrong_state_error() {
        let mut machine = HandshakeMachine::new();
        assert!(
            machine
                .on_command_catalog_page(sample_command_catalog_page())
                .is_err()
        );
    }

    #[test]
    fn test_identity_ack_after_identity_received() {
        let mut machine = HandshakeMachine::new();
        machine.on_identity(sample_identity()).unwrap();
        assert_eq!(*machine.state(), HandshakeState::WaitingCommandCatalog);

        // Identity ack is valid once identity has been received
        machine
            .on_host_ack(&HostAck {
                stage: AckStage::Identity,
                status: 0,
                message: "OK".to_string(),
            })
            .unwrap();
        // State should not change
        assert_eq!(*machine.state(), HandshakeState::WaitingCommandCatalog);
    }

    #[test]
    fn test_identity_ack_before_identity_received() {
        let mut machine = HandshakeMachine::new();
        // Identity ack before identity is received should fail
        assert!(
            machine
                .on_host_ack(&HostAck {
                    stage: AckStage::Identity,
                    status: 0,
                    message: "OK".to_string(),
                })
                .is_err()
        );
    }

    #[test]
    fn test_command_catalog_ack_after_catalog_received() {
        let mut machine = HandshakeMachine::new();
        machine.on_identity(sample_identity()).unwrap();
        machine
            .on_command_catalog_page(sample_command_catalog_page())
            .unwrap();
        assert_eq!(*machine.state(), HandshakeState::WaitingVariableCatalog);

        // CommandCatalog ack is valid once command catalog has been received
        machine
            .on_host_ack(&HostAck {
                stage: AckStage::CommandCatalog,
                status: 0,
                message: "OK".to_string(),
            })
            .unwrap();
        // State should not change
        assert_eq!(*machine.state(), HandshakeState::WaitingVariableCatalog);
    }

    #[test]
    fn test_command_catalog_ack_before_catalog_received() {
        let mut machine = HandshakeMachine::new();
        machine.on_identity(sample_identity()).unwrap();
        // CommandCatalog ack before catalog is received should fail
        assert!(
            machine
                .on_host_ack(&HostAck {
                    stage: AckStage::CommandCatalog,
                    status: 0,
                    message: "OK".to_string(),
                })
                .is_err()
        );
    }

    #[test]
    fn test_host_ack_error_status() {
        let mut machine = HandshakeMachine::new();
        machine.on_identity(sample_identity()).unwrap();
        machine
            .on_command_catalog_page(sample_command_catalog_page())
            .unwrap();
        machine
            .on_variable_catalog_page(sample_variable_catalog_page())
            .unwrap();

        // Host ack with error status should fail
        assert!(
            machine
                .on_host_ack(&HostAck {
                    stage: AckStage::VariableCatalog,
                    status: 1,
                    message: "rejected".to_string(),
                })
                .is_err()
        );
        // State should remain WaitingHostAck
        assert_eq!(*machine.state(), HandshakeState::WaitingHostAck);
    }

    #[test]
    fn test_streaming_ack_before_ready() {
        let mut machine = HandshakeMachine::new();
        machine.on_identity(sample_identity()).unwrap();
        // Streaming ack before ReadyToStream should fail
        assert!(
            machine
                .on_host_ack(&HostAck {
                    stage: AckStage::Streaming,
                    status: 0,
                    message: "OK".to_string(),
                })
                .is_err()
        );
    }
}
