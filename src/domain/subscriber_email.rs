use validator::validate_email;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<SubscriberEmail, String> {
        if validate_email(&s) {
            Ok(Self(s))
        } else {
            Err(format!("{s} is not a valid email address."))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::SubscriberEmail;
    use claims::assert_err;
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;
    use proptest::prelude::*;
    use proptest::test_runner::{RngAlgorithm, TestRng};

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursuladomain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    fn valid_email_strategy() -> impl proptest::strategy::Strategy<Value = String> {
        any::<[u8; 32]>().prop_map(|v| {
            let mut rng = TestRng::from_seed(RngAlgorithm::ChaCha, &v);
            SafeEmail().fake_with_rng(&mut rng)
        })
    }
    proptest! {
        #[test]
        fn valid_emails_are_parsed_successfully(valid_email in valid_email_strategy()) {
            dbg!(&valid_email);
            prop_assert!(SubscriberEmail::parse(valid_email).is_ok());
        }
    }
}
