// Helper structures and functions
#[derive(Default)]
pub struct AuthState {
    pub id: Option<String>,
    pub password: Option<String>,
}

impl AuthState {
    pub fn authenticate(&mut self, id: String, password: String) {
        self.id = Some(id);
        self.password = Some(password);
    }

    pub fn is_authenticated(&self) -> bool {
        self.id.is_some() && self.password.is_some()
    }

    pub fn is_match(&self, id: &str, password: &str) -> bool {
        match (&self.id, &self.password) {
            (Some(a), Some(b)) => a == id && b == password,
            _ => false,
        }
    }

    pub fn take_credentials(&mut self) -> Option<(String, String)> {
        if self.is_authenticated() {
            Some((self.id.take().unwrap(), self.password.take().unwrap()))
        } else {
            None
        }
    }
}
