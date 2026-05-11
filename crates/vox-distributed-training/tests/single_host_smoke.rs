use sha3::Digest;
use vox_crypto::{generate_signing_keypair, signing_key_from_bytes, signing_key_to_bytes};
use vox_distributed_training::{
    Batch, CheckpointBundle, DataParallelSession, GradientShard, SessionId, TrainingSession,
};

#[tokio::test]
async fn single_host_step_checkpoint_resume() {
    let (sk, vk) = generate_signing_keypair();
    let sk_dup = signing_key_from_bytes(&signing_key_to_bytes(&sk));
    let sid = SessionId::new();
    let mut sess = DataParallelSession::new(sid, 0, 1, sk, vk.clone());

    sess.step(Batch { batch_id: 1 }).await.expect("step");

    let tensor_hash = {
        let mut h = sha3::Sha3_512::new();
        h.update(b"grad-smoke");
        h.finalize().into()
    };
    let shard = GradientShard::sign(&sk_dup, sid, sess.step_index(), 0, tensor_hash);
    sess.all_reduce(shard).await.expect("all_reduce");

    let ckpt: CheckpointBundle = sess.checkpoint().await.expect("checkpoint");
    assert!(ckpt.verify(&vk));
    let op = ckpt.to_operation_kind();
    let round = serde_json::to_string(&op).expect("json");
    let back = serde_json::from_str(&round).expect("parse");
    assert_eq!(op, back);

    let sk_resume = signing_key_from_bytes(&signing_key_to_bytes(&sk_dup));
    let vk_resume = vox_crypto::to_verifying_key(&sk_resume);
    let mut recovered = DataParallelSession::new(sid, 0, 1, sk_resume, vk_resume);
    recovered.resume(&ckpt).await.expect("resume");
    assert_eq!(recovered.step_index(), ckpt.step);
}
