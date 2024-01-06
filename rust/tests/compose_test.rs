use async_socket_macros::{AsyncSocketEmitters, AsyncSocketListeners, AsyncSocketResponders};

fn check_composes<
  T: ::serde::ser::Serialize + ::serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
>(
  val: &T,
) {
  let ser = serde_json::to_string(&val).unwrap();
  let de: T = serde_json::from_str(&ser).unwrap();
  assert_eq!(val, &de);
}

fn check_composes_str<
  T: ::serde::ser::Serialize + ::serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
>(
  ser: &str,
) {
  let de: T = serde_json::from_str(ser).unwrap();
  let re_ser = serde_json::to_string(&de).unwrap();
  assert_eq!(ser, re_ser);
}

#[test]
fn compose_simple() {
  #[derive(AsyncSocketEmitters, AsyncSocketListeners, PartialEq, Eq, Debug)]
  enum Test {
    TestMsg1 { id: u64 },
  }

  check_composes(&Test::TestMsg1 { id: 45 });
  check_composes_str::<Test>(r#"{"event":"test_msg1","args":[10]}"#);
}

#[test]
fn compose_large() {
  #[derive(AsyncSocketEmitters, AsyncSocketListeners, PartialEq, Eq, Debug)]
  enum Test {
    TestMsg1 {
      id: [u64; 8],
      name: String,
      tag: bool,
    },
  }

  check_composes(&Test::TestMsg1 {
    id: [1, 2, 3, 4, 8, 10, 200, 19392],
    name: "test message!".to_string(),
    tag: true,
  });
  check_composes_str::<Test>(
    r#"{"event":"test_msg1","args":[[8,16,24,32,40,48,56,64],"new string",false]}"#,
  );
}

#[test]
fn compose_composite() {
  #[derive(::serde::Serialize, ::serde::Deserialize, PartialEq, Eq, Debug)]
  struct T1 {
    v1: u16,
    v2: bool,
    v3: (String, u64),
  }

  #[derive(AsyncSocketEmitters, AsyncSocketListeners, PartialEq, Eq, Debug)]
  enum EnumName {
    MessageOne { t1: T1, t2: bool },
  }

  check_composes(&EnumName::MessageOne {
    t1: T1 {
      v1: 84,
      v2: true,
      v3: ("hi bokus".to_string(), 1023),
    },
    t2: false,
  });
  check_composes_str::<EnumName>(
    r#"{"event":"message_one","args":[{"v1":101,"v2":false,"v3":["new message",1010101]},true]}"#,
  );
}

#[test]
fn compose_multiple_messages() {
  #[derive(AsyncSocketEmitters, AsyncSocketListeners, PartialEq, Eq, Debug)]
  enum NewMessage {
    First { id: String },
    Second { json: String },
    Third { done: bool },
  }

  check_composes(&NewMessage::First {
    id: "1209fj0d-90-fd".to_string(),
  });
  check_composes_str::<NewMessage>(r#"{"event":"first","args":["big string"]}"#);

  check_composes(&NewMessage::Second {
    json: "{ \"is this a test\": true }".to_string(),
  });
  check_composes_str::<NewMessage>(r#"{"event":"second","args":["another string"]}"#);

  check_composes(&NewMessage::Third { done: true });
  check_composes_str::<NewMessage>(r#"{"event":"third","args":[false]}"#);
}

#[test]
fn responder_simple() {
  #[derive(AsyncSocketResponders, Debug)]
  enum Test {
    TestMsg1 { id: u64 },
  }

  let ser = serde_json::to_string(&Test::TestMsg1 { id: 45 }).unwrap();
  assert_eq!(ser, r#"{"id":45}"#);
}

#[test]
fn responder_large() {
  #[derive(AsyncSocketResponders, Debug)]
  enum Test {
    TestMsg1 {
      id: [u64; 8],
      name: String,
      tag: bool,
    },
  }

  let ser = serde_json::to_string(&Test::TestMsg1 {
    id: [1, 2, 3, 4, 8, 10, 200, 19392],
    name: "test message!".to_string(),
    tag: true,
  })
  .unwrap();
  assert_eq!(
    ser,
    r#"{"id":[1,2,3,4,8,10,200,19392],"name":"test message!","tag":true}"#
  );
}

#[test]
fn responder_composite() {
  #[derive(::serde::Serialize, Debug)]
  struct T1 {
    v1: u16,
    v2: bool,
    v3: (String, u64),
  }

  #[derive(AsyncSocketResponders, Debug)]
  enum EnumName {
    MessageOne { t1: T1, t2: bool },
  }

  let ser = serde_json::to_string(&EnumName::MessageOne {
    t1: T1 {
      v1: 84,
      v2: true,
      v3: ("hi bokus".to_string(), 1023),
    },
    t2: false,
  })
  .unwrap();
  assert_eq!(
    ser,
    r#"{"t1":{"v1":84,"v2":true,"v3":["hi bokus",1023]},"t2":false}"#,
  );
}

#[test]
fn responders_multiple_messages() {
  #[derive(AsyncSocketResponders, Debug)]
  enum NewMessage {
    First { id: String },
    Second { json: String },
    Third { done: bool },
  }

  let ser = serde_json::to_string(&NewMessage::First {
    id: "1209fj0d-90-fd".to_string(),
  })
  .unwrap();
  assert_eq!(ser, r#"{"id":"1209fj0d-90-fd"}"#);

  let ser = serde_json::to_string(&NewMessage::Second {
    json: "{ \"is this a test\": true }".to_string(),
  })
  .unwrap();
  assert_eq!(ser, r#"{"json":"{ \"is this a test\": true }"}"#);

  let ser = serde_json::to_string(&NewMessage::Third { done: true }).unwrap();
  assert_eq!(ser, r#"{"done":true}"#);
}
