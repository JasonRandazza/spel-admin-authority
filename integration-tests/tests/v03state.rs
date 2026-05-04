use admin_authority::{AdminAuthority, AdminKey};
use admin_authority_sample::Instruction;
use admin_authority_sample_methods::{ADMIN_AUTHORITY_SAMPLE_ELF, ADMIN_AUTHORITY_SAMPLE_ID};
use borsh::BorshDeserialize;
use nssa::{
    PrivateKey, PublicKey, V03State,
    program::Program,
    program_deployment_transaction::{Message as DeployMessage, ProgramDeploymentTransaction},
    public_transaction::{Message, PublicTransaction, WitnessSet},
};
use nssa_core::account::{AccountId, Nonce};
use spel_framework::pda::{compute_pda, seed_from_str};

fn pda(seed: &str) -> AccountId {
    let seed_bytes = seed_from_str(seed);
    compute_pda(&ADMIN_AUTHORITY_SAMPLE_ID, &[&seed_bytes])
}

fn deploy_sample(state: &mut V03State) {
    let program = Program::new(ADMIN_AUTHORITY_SAMPLE_ELF.to_vec())
        .expect("admin_authority_sample ELF must be valid");
    assert_eq!(
        program.id(),
        ADMIN_AUTHORITY_SAMPLE_ID,
        "ELF image ID must match compiled constant"
    );
    let tx =
        ProgramDeploymentTransaction::new(DeployMessage::new(ADMIN_AUTHORITY_SAMPLE_ELF.to_vec()));
    state
        .transition_from_program_deployment_transaction(&tx)
        .unwrap();
}

#[test]
fn v03state_initialize_sets_admin_and_config() {
    let mut state = V03State::new_with_genesis_accounts(&[], vec![], 0);
    deploy_sample(&mut state);

    let admin_key = PrivateKey::try_new([1u8; 32]).unwrap();
    let admin_id = AccountId::from(&PublicKey::new_from_private_key(&admin_key));

    let msg = Message::try_new(
        ADMIN_AUTHORITY_SAMPLE_ID,
        vec![pda("admin_authority"), pda("config"), admin_id],
        vec![Nonce(0)],
        Instruction::Initialize { value: 42 },
    )
    .unwrap();
    let tx = PublicTransaction::new(msg.clone(), WitnessSet::for_message(&msg, &[&admin_key]));
    state.transition_from_public_transaction(&tx, 1, 0).unwrap();

    let authority = AdminAuthority::decode(
        state
            .get_account_by_id(pda("admin_authority"))
            .data
            .as_ref(),
    )
    .unwrap();
    assert_eq!(authority.admin, Some(AdminKey::Signer(admin_id)));
    assert!(!authority.revoked);

    let config = admin_authority_sample::Config::try_from_slice(
        state.get_account_by_id(pda("config")).data.as_ref(),
    )
    .unwrap();
    assert_eq!(config.value, 42);
}

#[test]
fn v03state_update_config_succeeds_for_admin() {
    let mut state = V03State::new_with_genesis_accounts(&[], vec![], 0);
    deploy_sample(&mut state);

    let admin_key = PrivateKey::try_new([1u8; 32]).unwrap();
    let admin_id = AccountId::from(&PublicKey::new_from_private_key(&admin_key));

    let init_msg = Message::try_new(
        ADMIN_AUTHORITY_SAMPLE_ID,
        vec![pda("admin_authority"), pda("config"), admin_id],
        vec![Nonce(0)],
        Instruction::Initialize { value: 1 },
    )
    .unwrap();
    state
        .transition_from_public_transaction(
            &PublicTransaction::new(
                init_msg.clone(),
                WitnessSet::for_message(&init_msg, &[&admin_key]),
            ),
            1,
            0,
        )
        .unwrap();

    // Admin nonce is now 1 after the initialize transaction.
    let update_msg = Message::try_new(
        ADMIN_AUTHORITY_SAMPLE_ID,
        vec![pda("admin_authority"), pda("config"), admin_id],
        vec![Nonce(1)],
        Instruction::UpdateConfig { value: 99 },
    )
    .unwrap();
    state
        .transition_from_public_transaction(
            &PublicTransaction::new(
                update_msg.clone(),
                WitnessSet::for_message(&update_msg, &[&admin_key]),
            ),
            2,
            0,
        )
        .unwrap();

    let config = admin_authority_sample::Config::try_from_slice(
        state.get_account_by_id(pda("config")).data.as_ref(),
    )
    .unwrap();
    assert_eq!(config.value, 99);
}
