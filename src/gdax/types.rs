// // Request
// // Subscribe to ETH-USD and ETH-EUR with the level2, heartbeat and ticker channels,
// // plus receive the ticker entries for ETH-BTC and ETH-USD
// {
//     "type": "subscribe",
//     "product_ids": [
//         "ETH-USD",
//         "ETH-EUR"
//     ],
//     "channels": [
//         "level2",
//         "heartbeat",
//         {
//             "name": "ticker",
//             "product_ids": [
//                 "ETH-BTC",
//                 "ETH-USD"
//             ]
//         }
//     ]
// }

pub struct Subscription {
    kind: String,
    product_ids: Vec<String>,
    channels: Vec<String>
}

// "type": "received",
// "time": "2014-11-07T08:19:27.028459Z",
// "product_id": "BTC-USD",
// "sequence": 10,
// "order_id": "d50ec984-77a8-460a-b958-66f114b0de9b",
// "size": "1.34",
// "price": "502.1",
// "side": "buy",
// "order_type": "limit"

pub struct Received {
kind: String,
time: String,
product_id: String,
sequence: u32,
order_id: String,
size: String,
price: String,
side: String,
order_type: String
}

//   "type": "open",
//   "time": "2014-11-07T08:19:27.028459Z",
//   "product_id": "BTC-USD",
//   "sequence": 10,
//   "order_id": "d50ec984-77a8-460a-b958-66f114b0de9b",
//   "price": "200.2",
//   "remaining_size": "1.00",
//   "side": "sell"

pub struct Open {
kind: String,
time: String,
product_id: String,
sequence: u32,
order_id: String,
price: String,
remaining_size: String,
side: String
}

//   "type": "done",
//   "time": "2014-11-07T08:19:27.028459Z",
//   "product_id": "BTC-USD",
//   "sequence": 10,
//   "price": "200.2",
//   "order_id": "d50ec984-77a8-460a-b958-66f114b0de9b",
//   "reason": "filled", // or "canceled"
//   "side": "sell",
//   "remaining_size": "0"

pub struct Done {
kind: String,
time: String,
product_id: String,
sequence: u32,
price: String,
order_id: String,
reason: String,
side: String,
remaining_size: String
}

//   "type": "match",
//   "trade_id": 10,
//   "sequence": 50,
//   "maker_order_id": "ac928c66-ca53-498f-9c13-a110027a60e8",
//   "taker_order_id": "132fb6ae-456b-4654-b4e0-d681ac05cea1",
//   "time": "2014-11-07T08:19:27.028459Z",
//   "product_id": "BTC-USD",
//   "size": "5.23512",
//   "price": "400.23",
//   "side": "sell"

pub struct Match {
kind: String,
trade_id: u32,
sequence: u32,
maker_order_id: String,
taker_order_id: String,
time: String,
product_id: String,
size: String,
price: String,
side: String
}

//   "type": "change",
//   "time": "2014-11-07T08:19:27.028459Z",
//   "sequence": 80,
//   "order_id": "ac928c66-ca53-498f-9c13-a110027a60e8",
//   "product_id": "BTC-USD",
//   "new_size": "5.23512",
//   "old_size": "12.234412",
//   "price": "400.23",
//   "side": "sell"

pub struct Change {
kind: String,
time: String,
sequence: u32,
order_id: String,
product_id: String,
new_size: String,
old_size: String,
price: String,
side: String
}

// "type": "activate",
// "product_id": "test-product",
// "timestamp": "1483736448.299000",
// "user_id": "12",
// "profile_id": "30000727-d308-cf50-7b1c-c06deb1934fc",
// "order_id": "7b52009b-64fd-0a2a-49e6-d8a939753077",
// "stop_type": "entry",
// "side": "buy",
// "stop_price": "80",
// "size": "2",
// "funds": "50",
// "taker_fee_rate": "0.0025",
// "private": true

pub struct Activate {
kind: String,
product_id: String,
timestamp: String,
user_id: String,
profile_id: String,
order_id: String,
stop_type: String,
side: String,
stop_price: String,
size: String,
funds: String,
taker_fee_rate: String,
private: String
}

// // Heartbeat message
// {
//     "type": "heartbeat",
//     "sequence": 90,
//     "last_trade_id": 20,
//     "product_id": "BTC-USD",
//     "time": "2014-11-07T08:19:28.464459Z"
// }

pub struct Heartbeat {
    kind: String,
    sequence: u32,
    last_trade_id: u32,
    product_id: String,
    time: String
}
