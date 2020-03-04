/// Represents the user info component of a URI.
/// ```notrust
/// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
///       |---------------|
///               |
///           user info
/// ```
#[derive(Debug)]
pub struct UserInfo<'a> {
    username: Option<&'a str>,
    password: Option<&'a str>,
}

impl<'a> UserInfo<'a> {
    pub(crate) fn new(username: Option<&'a str>, password: Option<&'a str>) -> Self {
        Self { username, password }
    }

    /// Get the username of this `UserInfo`.
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    ///       |------|
    ///           |
    ///       username
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::uri::*;
    /// let authority: Authority = "root@example.org:80".parse().unwrap();
    ///
    /// assert_eq!(authority.user_info().username(), Some("root"));
    /// ```
    pub fn username(&self) -> Option<&str> {
        self.username
    }

    /// Get the password of this `Authority`.
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    ///                |------|
    ///                    |
    ///                password
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::uri::*;
    /// let authority: Authority = "root:mypassword@example.org:80".parse().unwrap();
    ///
    /// assert_eq!(authority.user_info().password(), Some("mypassword"));
    /// ```
    pub fn password(&self) -> Option<&str> {
        self.password
    }
}
