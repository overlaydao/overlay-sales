use concordium_std::{
    collections::BTreeMap, fmt::Debug, schema, AccountAddress, Address, PublicKeyEd25519,
    SchemaType, Serial, Write,
};

pub mod error;
pub mod types;

pub const PRIVATE_RIDO_FEE: u8 = 10;
pub const PUBLIC_RIDO_FEE: u8 = 10;
pub const PUBLIC_RIDO_FEE_OVL: u8 = 5;
pub const PUBLIC_RIDO_FEE_BBB: u8 = 5;

// ---------------------------------------

/// Tag for the OvlClaim event.
pub const OVL_CLAIM_EVENT_TAG: u8 = 1u8;
pub const REGISTRATION_EVENT_TAG: u8 = 2u8;

/// A OvlClaimEvent
#[derive(Serial, SchemaType, Debug)]
pub struct ClaimEvent {
    pub to: Address,
    pub amount: u64,
    pub inc: u8,
}

/// The RegistrationEvent is logged when a new public key is registered.
#[derive(Debug, Serial, SchemaType)]
pub struct RegistrationEvent {
    pub account: AccountAddress,
    pub public_key: PublicKeyEd25519,
}

/// Tagged events to be serialized for the event log.
#[derive(Debug)]
pub enum OvlSaleEvent {
    Claim(ClaimEvent),
    Registration(RegistrationEvent),
}

impl Serial for OvlSaleEvent {
    fn serial<W: Write>(&self, out: &mut W) -> Result<(), W::Err> {
        match self {
            OvlSaleEvent::Claim(event) => {
                out.write_u8(OVL_CLAIM_EVENT_TAG)?;
                event.serial(out)
            }
            OvlSaleEvent::Registration(event) => {
                out.write_u8(REGISTRATION_EVENT_TAG)?;
                event.serial(out)
            }
        }
    }
}

impl schema::SchemaType for OvlSaleEvent {
    fn get_type() -> schema::Type {
        let mut event_map = BTreeMap::new();
        event_map.insert(
            OVL_CLAIM_EVENT_TAG,
            (
                "Claim".to_string(),
                schema::Fields::Named(vec![
                    (String::from("to"), Address::get_type()),
                    (String::from("amount"), u64::get_type()),
                    (String::from("inc"), u8::get_type()),
                ]),
            ),
        );
        event_map.insert(
            REGISTRATION_EVENT_TAG,
            (
                "Registration".to_string(),
                schema::Fields::Named(vec![
                    (String::from("account"), AccountAddress::get_type()),
                    (String::from("public_key"), PublicKeyEd25519::get_type()),
                ]),
            ),
        );
        schema::Type::TaggedEnum(event_map)
    }
}
