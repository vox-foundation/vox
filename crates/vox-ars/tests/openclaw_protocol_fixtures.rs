use vox_ars::openclaw_protocol::InboundFrame;

#[test]
fn parses_connect_challenge_fixture() {
    let raw = include_str!("../../../contracts/openclaw/protocol/connect.challenge.json");
    let frame: InboundFrame = serde_json::from_str(raw).expect("parse challenge fixture");
    match frame {
        InboundFrame::Event { event, .. } => assert_eq!(event, "connect.challenge"),
        _ => panic!("expected event frame"),
    }
}

#[test]
fn parses_hello_ok_fixture() {
    let raw = include_str!("../../../contracts/openclaw/protocol/connect.hello-ok.json");
    let frame: InboundFrame = serde_json::from_str(raw).expect("parse hello-ok fixture");
    match frame {
        InboundFrame::Response { ok, payload, .. } => {
            assert!(ok);
            assert_eq!(
                payload
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
                "hello-ok"
            );
        }
        _ => panic!("expected response frame"),
    }
}

#[test]
fn parses_subscriptions_response_fixture() {
    let raw = include_str!("../../../contracts/openclaw/protocol/subscriptions.list.response.json");
    let frame: InboundFrame = serde_json::from_str(raw).expect("parse subscriptions fixture");
    match frame {
        InboundFrame::Response { ok, payload, .. } => {
            assert!(ok);
            let items = payload
                .get("subscriptions")
                .and_then(serde_json::Value::as_array)
                .expect("subscriptions array");
            assert!(!items.is_empty());
        }
        _ => panic!("expected response frame"),
    }
}
