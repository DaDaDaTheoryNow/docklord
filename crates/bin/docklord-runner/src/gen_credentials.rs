use rand::{
    Rng,
    distr::{Alphanumeric, SampleString},
    rng,
};

pub fn generate_node_id() -> String {
    Alphanumeric.sample_string(&mut rng(), 16)
}

pub fn generate_secure_password() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789\
                            !*";

    let mut rng = rng();
    (0..24)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
