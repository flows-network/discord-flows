#[cfg(test)]
mod tests {
    use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Claim {
        flows_user: String,
        exp: usize,
    }

    #[test]
    fn it_works() {
        let now = std::time::SystemTime::now();
        let claim = Claim {
            flows_user: String::from("DarumaDocker"),
            exp: now
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as usize,
        };
        let private_key = std::env::var("FLOWS_JWT_PRIVATE_KEY").unwrap();
        let token = encode(
            &Header::new(Algorithm::RS256),
            &claim,
            &EncodingKey::from_rsa_pem(private_key.as_bytes()).unwrap(),
        )
        .unwrap();
        println!("{}", token);

        let mut val = Validation::new(Algorithm::RS256);
        val.leeway = 60;
        let public_key = std::env::var("FLOWS_JWT_PUBLIC_KEY").unwrap();
        let x = decode::<Claim>(
            &token,
            &DecodingKey::from_rsa_pem(public_key.as_bytes()).unwrap(),
            &val,
        )
        .unwrap();

        assert_eq!(claim, x.claims);
    }
}
