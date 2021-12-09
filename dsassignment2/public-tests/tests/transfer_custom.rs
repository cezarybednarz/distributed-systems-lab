use assignment_2_solution::{
  deserialize_register_command, serialize_register_command, ClientCommandHeader,
  ClientRegisterCommand, ClientRegisterCommandContent, RegisterCommand, SystemCommandHeader,
  SystemRegisterCommand, SystemRegisterCommandContent,
};
use assignment_2_test_utils::transfer::*;
use ntest::timeout;
use uuid::Uuid;

#[tokio::test]
#[timeout(200)]
async fn deserialized_read_has_correct_format() {
  unimplemented!();
}
