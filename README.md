To test with executable (better performance):

Open terminal window:
  cd ~/number_server
  cargo build --release
  ./target/release/number_server

In a different terminal (up to 5): 
  cd ~/sender_test
  cargo build --release
  ./target/release/sender_test
