use strum_macros::EnumIter;

#[derive(EnumIter, PartialEq, Debug)]
pub enum SubscriberState {
    Active,
    Pending,
    Disabled,
}

impl SubscriberState {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubscriberState::Active => "active",
            SubscriberState::Pending => "pending_confirmation",
            SubscriberState::Disabled => "disabled",
        }
    }
}

impl TryFrom<String> for SubscriberState {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "pending_confirmation" => Ok(Self::Pending),
            "disabled" => Ok(Self::Disabled),
            other => Err(format!("Unknown subscriber status: {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberState;
    use claims::{assert_err, assert_ok};
    use strum::IntoEnumIterator;

    #[test]
    pub fn test_try_from_covers_all_values() {
        for s in SubscriberState::iter() {
            let state_str: String = s.as_str().into();
            let s_from_str = SubscriberState::try_from(state_str);

            assert_ok!(&s_from_str);
            assert_eq!(s, s_from_str.unwrap());
        }

        let s_from_str = SubscriberState::try_from("AcTive".to_owned());
        assert_ok!(&s_from_str);
    }

    #[test]
    pub fn test_try_from_returns_error_for_unknown_status() {
        let s_from_str = SubscriberState::try_from("undefined".to_owned());
        assert_err!(&s_from_str);
    }
}
