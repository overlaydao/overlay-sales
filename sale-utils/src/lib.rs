use concordium_std::{
    collections::BTreeMap, fmt::Debug, schema, Address, SchemaType, Serial, Write,
};

pub mod error;
pub mod types;

pub const PRIVATE_RIDO_FEE: u8 = 10;
pub const PUBLIC_RIDO_FEE: u8 = 10;
pub const PUBLIC_RIDO_FEE_OVL: u8 = 5;
pub const PUBLIC_RIDO_FEE_BBB: u8 = 5;

// ---------------------------------------

/// Tag for the OvlClaim event.
pub const OVL_CLAIM_EVENT_TAG: u8 = 1;

/// A OvlClaimEvent
#[derive(Serial, SchemaType, Debug)]
pub struct ClaimEvent {
    pub to: Address,
    pub amount: u64,
    pub inc: u8,
}

/// Tagged events to be serialized for the event log.
#[derive(Debug)]
pub enum OvlSaleEvent {
    Claim(ClaimEvent),
}

impl Serial for OvlSaleEvent {
    fn serial<W: Write>(&self, out: &mut W) -> Result<(), W::Err> {
        match self {
            OvlSaleEvent::Claim(event) => {
                out.write_u8(OVL_CLAIM_EVENT_TAG)?;
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
        schema::Type::TaggedEnum(event_map)
    }
}
