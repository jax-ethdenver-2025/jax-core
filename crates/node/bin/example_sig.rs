use iroh::SecretKey;

fn main() {
   let zeros = [0u8; 32];
   let key = SecretKey::from_bytes(&zeros);
   let node_id = key.public();
   let node_id_bytes = node_id.as_bytes();
   let sig = key.sign(node_id_bytes);
   let sig_bytes = sig.to_bytes();
   let s_bytes = sig.s_bytes();
   let r_bytes = sig.r_bytes();
   let sig_hex = hex::encode(sig_bytes);
   let k_hex = hex::encode(node_id_bytes);
   let s_hex = hex::encode(s_bytes);
   let r_hex = hex::encode(r_bytes);
   let m_hex = hex::encode(sig_bytes);
   println!("Node ID: {}", node_id);
   println!("Signature: {}", sig_hex);
   println!("k: {}", k_hex);
   println!("s: {}", s_hex);
   println!("r: {}", r_hex);
   println!("m: {}", m_hex);
}