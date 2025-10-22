#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use serde_evolve::Versioned;

mod renamed_representation {
    use super::*;

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct AccountV1 {
        pub username: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct AccountV2 {
        pub username: String,
        pub is_active: bool,
    }

    impl From<AccountV1> for AccountV2 {
        fn from(v1: AccountV1) -> Self {
            Self {
                username: v1.username,
                is_active: true,
            }
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Versioned)]
    #[versioned(mode = "infallible", rep = AccountEnvelope, chain(AccountV1, AccountV2))]
    pub struct Account {
        pub username: String,
        pub is_active: bool,
    }

    impl From<AccountV2> for Account {
        fn from(v2: AccountV2) -> Self {
            Self {
                username: v2.username,
                is_active: v2.is_active,
            }
        }
    }

    impl From<&Account> for AccountV2 {
        fn from(account: &Account) -> Self {
            Self {
                username: account.username.clone(),
                is_active: account.is_active,
            }
        }
    }

    #[test]
    fn renamed_representation_supports_round_trip() {
        let json_v1 = r#"{"_version":"1","username":"trinity"}"#;
        let rep: AccountEnvelope = serde_json::from_str(json_v1).unwrap();
        assert_eq!(rep.version(), 1);

        let account: Account = rep.into();
        assert_eq!(account.username, "trinity");
        assert!(account.is_active);

        let rep_latest = AccountEnvelope::from(&account);
        assert!(rep_latest.is_current());
        assert_eq!(AccountEnvelope::CURRENT, 2);

        let json = serde_json::to_string(&rep_latest).unwrap();
        let rep_round: AccountEnvelope = serde_json::from_str(&json).unwrap();
        let account_round: Account = rep_round.into();
        assert_eq!(account_round, account);
    }
}

mod multi_version_chain {
    use super::*;
    use anyhow::Context;
    use std::convert::TryFrom;

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ProfileV1 {
        pub display_name: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ProfileV2 {
        pub given_name: String,
        pub family_name: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ProfileV3 {
        pub given_name: String,
        pub family_name: String,
        pub preferred: Option<String>,
    }

    impl TryFrom<ProfileV1> for ProfileV2 {
        type Error = anyhow::Error;

        fn try_from(v1: ProfileV1) -> Result<Self, Self::Error> {
            let mut parts = v1.display_name.splitn(2, ' ');
            let given = parts
                .next()
                .map(str::to_owned)
                .context("display_name missing given name")?;
            let family = parts
                .next()
                .map(str::to_owned)
                .context("display_name missing family name")?;

            Ok(Self {
                given_name: given,
                family_name: family,
            })
        }
    }

    impl From<ProfileV2> for ProfileV3 {
        fn from(v2: ProfileV2) -> Self {
            Self {
                given_name: v2.given_name,
                family_name: v2.family_name,
                preferred: None,
            }
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Versioned)]
    #[versioned(error = anyhow::Error, chain(ProfileV1, ProfileV2, ProfileV3))]
    pub struct Profile {
        pub given_name: String,
        pub family_name: String,
        pub preferred: Option<String>,
    }

    impl TryFrom<ProfileV3> for Profile {
        type Error = anyhow::Error;

        fn try_from(v3: ProfileV3) -> Result<Self, Self::Error> {
            Ok(Self {
                given_name: v3.given_name,
                family_name: v3.family_name,
                preferred: v3.preferred,
            })
        }
    }

    impl From<&Profile> for ProfileV3 {
        fn from(profile: &Profile) -> Self {
            Self {
                given_name: profile.given_name.clone(),
                family_name: profile.family_name.clone(),
                preferred: profile.preferred.clone(),
            }
        }
    }

    #[test]
    fn multi_version_chain_supports_conversions() {
        let json_v1 = r#"{"_version":"1","display_name":"Ada Lovelace"}"#;
        let rep_v1: ProfileVersions = serde_json::from_str(json_v1).unwrap();
        let profile_from_v1 = Profile::try_from(rep_v1).unwrap();
        assert_eq!(profile_from_v1.given_name, "Ada");
        assert_eq!(profile_from_v1.family_name, "Lovelace");
        assert_eq!(profile_from_v1.preferred, None);

        let json_v2 = r#"{"_version":"2","given_name":"Grace","family_name":"Hopper"}"#;
        let rep_v2: ProfileVersions = serde_json::from_str(json_v2).unwrap();
        let profile_from_v2 = Profile::try_from(rep_v2).unwrap();
        assert_eq!(profile_from_v2.given_name, "Grace");
        assert_eq!(profile_from_v2.family_name, "Hopper");

        let original = Profile {
            given_name: "Katherine".to_string(),
            family_name: "Johnson".to_string(),
            preferred: Some("Kat".to_string()),
        };
        let rep_latest = ProfileVersions::from(&original);
        assert!(rep_latest.is_current());
        assert_eq!(ProfileVersions::CURRENT, 3);

        let json = serde_json::to_string(&rep_latest).unwrap();
        let rep_round: ProfileVersions = serde_json::from_str(&json).unwrap();
        let profile_round = Profile::try_from(rep_round).unwrap();
        assert_eq!(profile_round, original);
    }
}

mod transparent_edge_cases {
    use super::*;
    use anyhow::Context;
    use std::convert::TryFrom;

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct PersonV1 {
        pub name: String,
        pub age: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct PersonV2 {
        pub name: String,
        pub age: u8,
        pub verified: bool,
    }

    impl TryFrom<PersonV1> for PersonV2 {
        type Error = anyhow::Error;

        fn try_from(v1: PersonV1) -> Result<Self, Self::Error> {
            let age = v1.age.parse::<u8>().context("age must be a number")?;
            Ok(Self {
                name: v1.name,
                age,
                verified: false,
            })
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Versioned)]
    #[versioned(
        error = anyhow::Error,
        rep = PersonEnvelope,
        chain(PersonV1, PersonV2),
        transparent = true
    )]
    pub struct Person {
        pub name: String,
        pub age: u8,
        pub verified: bool,
    }

    impl TryFrom<PersonV2> for Person {
        type Error = anyhow::Error;

        fn try_from(v2: PersonV2) -> Result<Self, Self::Error> {
            Ok(Self {
                name: v2.name,
                age: v2.age,
                verified: v2.verified,
            })
        }
    }

    impl From<&Person> for PersonV2 {
        fn from(person: &Person) -> Self {
            Self {
                name: person.name.clone(),
                age: person.age,
                verified: person.verified,
            }
        }
    }

    #[test]
    fn transparent_mode_handles_round_trip_and_errors() {
        let json_v1 = r#"{"_version":"1","name":"Iris","age":"29"}"#;
        let person: Person = serde_json::from_str(json_v1).unwrap();
        assert_eq!(person.name, "Iris");
        assert_eq!(person.age, 29);
        assert!(!person.verified);

        let json_v2 = r#"{"_version":"2","name":"Nia","age":31,"verified":true}"#;
        let rep_v2: PersonEnvelope = serde_json::from_str(json_v2).unwrap();
        let person_from_v2 = Person::try_from(rep_v2).unwrap();
        assert!(person_from_v2.verified);
        assert_eq!(person_from_v2.age, 31);

        let rep_latest = PersonEnvelope::from(&person);
        assert!(rep_latest.is_current());

        let json = serde_json::to_string(&person).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["_version"], "2");
        assert_eq!(parsed["name"], "Iris");

        let round_trip: Person = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip, person);

        let rep_round: PersonEnvelope = serde_json::from_str(&json).unwrap();
        let domain_round = Person::try_from(rep_round).unwrap();
        assert_eq!(domain_round, person);

        let invalid = r#"{"_version":"1","name":"Iris","age":"not-a-number"}"#;
        let err = serde_json::from_str::<Person>(invalid).unwrap_err();
        assert!(err.is_data());
    }
}
