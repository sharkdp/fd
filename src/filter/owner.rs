use anyhow::{anyhow, Result};
use std::fs;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OwnerFilter {
    uid: Option<u32>,
    gid: Option<u32>,
}

impl OwnerFilter {
    pub fn from_string(input: &str) -> Result<Self> {
        let mut it = input.split(':');
        let (fst, snd) = (it.next(), it.next());

        let uid = match fst {
            Some("") | None => None,
            Some(s) => {
                let maybe_uid = s
                    .parse()
                    .ok()
                    .or_else(|| users::get_user_by_name(s).map(|user| user.uid()));
                match maybe_uid {
                    Some(uid) => Some(uid),
                    _ => return Err(anyhow!("'{}' is not a recognized user name", s)),
                }
            }
        };
        let gid = match snd {
            Some("") | None => None,
            Some(s) => {
                let maybe_gid = s
                    .parse()
                    .ok()
                    .or_else(|| users::get_group_by_name(s).map(|group| group.gid()));
                match maybe_gid {
                    Some(gid) => Some(gid),
                    _ => return Err(anyhow!("'{}' is not a recognized group name", s)),
                }
            }
        };

        if uid.is_none() && gid.is_none() {
            Err(anyhow!(
                "'{}' is not a valid user/group specifier. See 'fd --help'.",
                input
            ))
        } else {
            Ok(OwnerFilter { uid, gid })
        }
    }

    pub fn matches(&self, md: &fs::Metadata) -> bool {
        use std::os::unix::fs::MetadataExt;

        let uid_ok = self.uid.map(|u| u == md.uid()).unwrap_or(true);
        let gid_ok = self.gid.map(|g| g == md.gid()).unwrap_or(true);

        uid_ok && gid_ok
    }
}

#[cfg(test)]
mod owner_parsing {
    use super::OwnerFilter;

    macro_rules! owner_tests {
        ($($name:ident: $value:expr => $result:pat,)*) => {
            $(
                #[test]
                fn $name() {
                    let o = OwnerFilter::from_string($value);
                    match o {
                        $result => {},
                        _ => panic!("{:?} does not match {}", o, stringify!($result)),
                    }
                }
            )*
        };
    }

    owner_tests! {
        empty:      ""      => Err(_),
        uid_only:   "5"     => Ok(OwnerFilter { uid: Some(5), gid: None   }),
        uid_gid:    "9:3"   => Ok(OwnerFilter { uid: Some(9), gid: Some(3)}),
        gid_only:   ":8"    => Ok(OwnerFilter { uid: None,    gid: Some(8)}),
        colon_only: ":"     => Err(_),
        trailing:   "5:"    => Ok(OwnerFilter { uid: Some(5), gid: None   }),
    }
}
