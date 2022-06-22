use http::field::*;
use http::*;

use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

use std::collections::HashMap;

#[test]
fn header_map_fuzz() {
    fn prop(fuzz: Fuzz) -> TestResult {
        fuzz.run();
        TestResult::from_bool(true)
    }

    QuickCheck::new().quickcheck(prop as fn(Fuzz) -> TestResult)
}

#[derive(Debug, Clone)]
struct Fuzz {
    // The magic seed that makes the test case reproducible
    seed: [u8; 32],

    // Actions to perform
    steps: Vec<Step>,

    // Number of steps to drop
    reduce: usize,
}

#[derive(Debug)]
struct Weight {
    insert: usize,
    remove: usize,
    append: usize,
}

#[derive(Debug, Clone)]
struct Step {
    action: Action,
    expect: AltMap,
}

#[derive(Debug, Clone)]
enum Action {
    Insert {
        name: FieldName,         // Name to insert
        val: FieldValue,         // Value to insert
        old: Option<FieldValue>, // Old value
    },
    Append {
        name: FieldName,
        val: FieldValue,
        ret: bool,
    },
    Remove {
        name: FieldName,         // Name to remove
        val: Option<FieldValue>, // Value to get
    },
}

// An alternate implementation of FieldMap backed by HashMap
#[derive(Debug, Clone, Default)]
struct AltMap {
    map: HashMap<FieldName, Vec<FieldValue>>,
}

impl Fuzz {
    fn new(seed: [u8; 32]) -> Fuzz {
        // Seed the RNG
        let mut rng = StdRng::from_seed(seed);

        let mut steps = vec![];
        let mut expect = AltMap::default();
        let num = rng.gen_range(5, 500);

        let weight = Weight {
            insert: rng.gen_range(1, 10),
            remove: rng.gen_range(1, 10),
            append: rng.gen_range(1, 10),
        };

        while steps.len() < num {
            steps.push(expect.gen_step(&weight, &mut rng));
        }

        Fuzz {
            seed: seed,
            steps: steps,
            reduce: 0,
        }
    }

    fn run(self) {
        // Create a new header map
        let mut map = FieldMap::new();

        // Number of steps to perform
        let take = self.steps.len() - self.reduce;

        for step in self.steps.into_iter().take(take) {
            step.action.apply(&mut map);

            step.expect.assert_identical(&map);
        }
    }
}

impl Arbitrary for Fuzz {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Fuzz::new(Rng::gen(g))
    }
}

impl AltMap {
    fn gen_step(&mut self, weight: &Weight, rng: &mut StdRng) -> Step {
        let action = self.gen_action(weight, rng);

        Step {
            action: action,
            expect: self.clone(),
        }
    }

    /// This will also apply the action against `self`
    fn gen_action(&mut self, weight: &Weight, rng: &mut StdRng) -> Action {
        let sum = weight.insert + weight.remove + weight.append;

        let mut num = rng.gen_range(0, sum);

        if num < weight.insert {
            return self.gen_insert(rng);
        }

        num -= weight.insert;

        if num < weight.remove {
            return self.gen_remove(rng);
        }

        num -= weight.remove;

        if num < weight.append {
            return self.gen_append(rng);
        }

        unreachable!();
    }

    fn gen_insert(&mut self, rng: &mut StdRng) -> Action {
        let name = self.gen_name(4, rng);
        let val = gen_header_value(rng);
        let old = self.insert(name.clone(), val.clone());

        Action::Insert {
            name: name,
            val: val,
            old: old,
        }
    }

    fn gen_remove(&mut self, rng: &mut StdRng) -> Action {
        let name = self.gen_name(-4, rng);
        let val = self.remove(&name);

        Action::Remove {
            name: name,
            val: val,
        }
    }

    fn gen_append(&mut self, rng: &mut StdRng) -> Action {
        let name = self.gen_name(-5, rng);
        let val = gen_header_value(rng);

        let vals = self.map.entry(name.clone()).or_insert(vec![]);

        let ret = !vals.is_empty();
        vals.push(val.clone());

        Action::Append {
            name: name,
            val: val,
            ret: ret,
        }
    }

    /// Negative numbers weigh finding an existing header higher
    fn gen_name(&self, weight: i32, rng: &mut StdRng) -> FieldName {
        let mut existing = rng.gen_ratio(1, weight.abs() as u32);

        if weight < 0 {
            existing = !existing;
        }

        if existing {
            // Existing header
            if let Some(name) = self.find_random_name(rng) {
                name
            } else {
                gen_header_name(rng)
            }
        } else {
            gen_header_name(rng)
        }
    }

    fn find_random_name(&self, rng: &mut StdRng) -> Option<FieldName> {
        if self.map.is_empty() {
            None
        } else {
            let n = rng.gen_range(0, self.map.len());
            self.map.keys().nth(n).map(Clone::clone)
        }
    }

    fn insert(&mut self, name: FieldName, val: FieldValue) -> Option<FieldValue> {
        let old = self.map.insert(name, vec![val]);
        old.and_then(|v| v.into_iter().next())
    }

    fn remove(&mut self, name: &FieldName) -> Option<FieldValue> {
        self.map.remove(name).and_then(|v| v.into_iter().next())
    }

    fn assert_identical(&self, other: &FieldMap<FieldValue>) {
        assert_eq!(self.map.len(), other.keys_len());

        for (key, val) in &self.map {
            // Test get
            assert_eq!(other.get(key), val.get(0));

            // Test get_all
            let vals = other.get_all(key);
            let actual: Vec<_> = vals.iter().collect();
            assert_eq!(&actual[..], &val[..]);
        }
    }
}

impl Action {
    fn apply(self, map: &mut FieldMap<FieldValue>) {
        match self {
            Action::Insert { name, val, old } => {
                let actual = map.insert(name, val);
                assert_eq!(actual, old);
            }
            Action::Remove { name, val } => {
                // Just to help track the state, load all associated values.
                let _ = map.get_all(&name).iter().collect::<Vec<_>>();

                let actual = map.remove(&name);
                assert_eq!(actual, val);
            }
            Action::Append { name, val, ret } => {
                assert_eq!(ret, map.append(name, val));
            }
        }
    }
}

fn gen_header_name(g: &mut StdRng) -> FieldName {
    const STANDARD_HEADERS: &'static [FieldName] = &[
        field::ACCEPT,
        field::ACCEPT_CHARSET,
        field::ACCEPT_ENCODING,
        field::ACCEPT_LANGUAGE,
        field::ACCEPT_RANGES,
        field::ACCESS_CONTROL_ALLOW_CREDENTIALS,
        field::ACCESS_CONTROL_ALLOW_HEADERS,
        field::ACCESS_CONTROL_ALLOW_METHODS,
        field::ACCESS_CONTROL_ALLOW_ORIGIN,
        field::ACCESS_CONTROL_EXPOSE_HEADERS,
        field::ACCESS_CONTROL_MAX_AGE,
        field::ACCESS_CONTROL_REQUEST_HEADERS,
        field::ACCESS_CONTROL_REQUEST_METHOD,
        field::AGE,
        field::ALLOW,
        field::ALT_SVC,
        field::AUTHORIZATION,
        field::CACHE_CONTROL,
        field::CONNECTION,
        field::CONTENT_DISPOSITION,
        field::CONTENT_ENCODING,
        field::CONTENT_LANGUAGE,
        field::CONTENT_LENGTH,
        field::CONTENT_LOCATION,
        field::CONTENT_RANGE,
        field::CONTENT_SECURITY_POLICY,
        field::CONTENT_SECURITY_POLICY_REPORT_ONLY,
        field::CONTENT_TYPE,
        field::COOKIE,
        field::DNT,
        field::DATE,
        field::ETAG,
        field::EXPECT,
        field::EXPIRES,
        field::FORWARDED,
        field::FROM,
        field::HOST,
        field::IF_MATCH,
        field::IF_MODIFIED_SINCE,
        field::IF_NONE_MATCH,
        field::IF_RANGE,
        field::IF_UNMODIFIED_SINCE,
        field::LAST_MODIFIED,
        field::LINK,
        field::LOCATION,
        field::MAX_FORWARDS,
        field::ORIGIN,
        field::PRAGMA,
        field::PROXY_AUTHENTICATE,
        field::PROXY_AUTHORIZATION,
        field::PUBLIC_KEY_PINS,
        field::PUBLIC_KEY_PINS_REPORT_ONLY,
        field::RANGE,
        field::REFERER,
        field::REFERRER_POLICY,
        field::REFRESH,
        field::RETRY_AFTER,
        field::SEC_WEBSOCKET_ACCEPT,
        field::SEC_WEBSOCKET_EXTENSIONS,
        field::SEC_WEBSOCKET_KEY,
        field::SEC_WEBSOCKET_PROTOCOL,
        field::SEC_WEBSOCKET_VERSION,
        field::SERVER,
        field::SET_COOKIE,
        field::STRICT_TRANSPORT_SECURITY,
        field::TE,
        field::TRAILER,
        field::TRANSFER_ENCODING,
        field::UPGRADE,
        field::UPGRADE_INSECURE_REQUESTS,
        field::USER_AGENT,
        field::VARY,
        field::VIA,
        field::WARNING,
        field::WWW_AUTHENTICATE,
        field::X_CONTENT_TYPE_OPTIONS,
        field::X_DNS_PREFETCH_CONTROL,
        field::X_FRAME_OPTIONS,
        field::X_XSS_PROTECTION,
    ];

    if g.gen_ratio(1, 2) {
        STANDARD_HEADERS.choose(g).unwrap().clone()
    } else {
        let value = gen_string(g, 1, 25);
        FieldName::from_bytes(value.as_bytes()).unwrap()
    }
}

fn gen_header_value(g: &mut StdRng) -> FieldValue {
    let value = gen_string(g, 0, 70);
    FieldValue::from_bytes(value.as_bytes()).unwrap()
}

fn gen_string(g: &mut StdRng, min: usize, max: usize) -> String {
    let bytes: Vec<_> = (min..max)
        .map(|_| {
            // Chars to pick from
            b"ABCDEFGHIJKLMNOPQRSTUVabcdefghilpqrstuvwxyz----"
                .choose(g)
                .unwrap()
                .clone()
        })
        .collect();

    String::from_utf8(bytes).unwrap()
}
