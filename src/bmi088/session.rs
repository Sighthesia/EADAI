/// BMI088 session handshake state machine.
use super::encoder::host_command_label;
use super::models::{
    Bmi088Frame, Bmi088HostCommand, Bmi088IdentityFrame, Bmi088SchemaFrame, Bmi088SessionPhase,
};

#[derive(Clone, Debug)]
pub struct Bmi088SessionState {
    phase: Bmi088SessionPhase,
    identity: Option<Bmi088IdentityFrame>,
    schema: Option<Bmi088SchemaFrame>,
    boot_commands_sent: bool,
}

impl Bmi088SessionState {
    pub fn new() -> Self {
        Self {
            phase: Bmi088SessionPhase::AwaitingSchema,
            identity: None,
            schema: None,
            boot_commands_sent: false,
        }
    }

    pub fn boot_commands(&mut self) -> Vec<Bmi088HostCommand> {
        if self.boot_commands_sent {
            return Vec::new();
        }

        self.boot_commands_sent = true;
        self.phase = Bmi088SessionPhase::AwaitingSchema;
        vec![Bmi088HostCommand::ReqSchema]
    }

    pub fn schema_retry_commands(&self) -> Vec<Bmi088HostCommand> {
        if self.phase == Bmi088SessionPhase::AwaitingSchema {
            vec![Bmi088HostCommand::ReqSchema]
        } else {
            Vec::new()
        }
    }

    pub fn phase(&self) -> Bmi088SessionPhase {
        self.phase
    }

    pub fn schema(&self) -> Option<&Bmi088SchemaFrame> {
        self.schema.as_ref()
    }

    pub fn identity(&self) -> Option<&Bmi088IdentityFrame> {
        self.identity.as_ref()
    }

    pub fn on_frame(&mut self, frame: &Bmi088Frame) -> Vec<Bmi088HostCommand> {
        let previous_phase = self.phase;
        match frame {
            Bmi088Frame::Identity(identity) => {
                self.identity = Some(identity.clone());
                eprintln!(
                    "[bmi088][session] rx IDENTITY seq={} phase={:?} device={} protocol={} schema_fields={} sample_len={}",
                    identity.seq,
                    previous_phase,
                    identity.device_name,
                    identity.protocol_version,
                    identity.schema_field_count,
                    identity.sample_payload_len,
                );
                Vec::new()
            }
            Bmi088Frame::ShellOutput(_) => Vec::new(),
            Bmi088Frame::Schema(schema) => {
                self.schema = Some(schema.clone());
                if matches!(
                    self.phase,
                    Bmi088SessionPhase::Streaming | Bmi088SessionPhase::Stopped
                ) {
                    eprintln!(
                        "[bmi088][session] rx SCHEMA seq={} phase={:?} field_count={} sample_len={} ignored_rehandshake=true",
                        schema.seq,
                        previous_phase,
                        schema.fields.len(),
                        schema.sample_len,
                    );
                    return Vec::new();
                }
                self.phase = Bmi088SessionPhase::AwaitingAck;
                eprintln!(
                    "[bmi088][session] rx SCHEMA seq={} phase={:?}->{:?} field_count={} sample_len={} next=ACK,START",
                    schema.seq,
                    previous_phase,
                    self.phase,
                    schema.fields.len(),
                    schema.sample_len,
                );
                vec![Bmi088HostCommand::Ack, Bmi088HostCommand::Start]
            }
            Bmi088Frame::Sample(_) => {
                self.phase = Bmi088SessionPhase::Streaming;
                eprintln!(
                    "[bmi088][session] rx SAMPLE phase={:?}->{:?}",
                    previous_phase, self.phase,
                );
                Vec::new()
            }
        }
    }

    pub fn on_host_command(&mut self, command: Bmi088HostCommand) {
        let previous_phase = self.phase;
        self.phase = match command {
            Bmi088HostCommand::Ack => Bmi088SessionPhase::AwaitingStart,
            Bmi088HostCommand::Start => Bmi088SessionPhase::Streaming,
            Bmi088HostCommand::Stop => Bmi088SessionPhase::Stopped,
            Bmi088HostCommand::ReqSchema => Bmi088SessionPhase::AwaitingSchema,
            Bmi088HostCommand::ReqIdentity
            | Bmi088HostCommand::ReqTuning
            | Bmi088HostCommand::SetTuning
            | Bmi088HostCommand::ShellExec => self.phase,
        };
        eprintln!(
            "[bmi088][session] tx {} phase={:?}->{:?}",
            host_command_label(&command),
            previous_phase,
            self.phase,
        );
    }
}

impl Default for Bmi088SessionState {
    fn default() -> Self {
        Self::new()
    }
}
