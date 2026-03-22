//! # QLoRA Training Example
//!
//! Demonstrates how to:
//! - Create a QLoRA trainer with training configuration
//! - Initialize layers with gradient tracking
//! - Run a training step with mock data
//! - Monitor training metrics
//!
//! This example shows the complete training loop setup for QLoRA, including:
//! - Proper parameter registration with VarBuilder
//! - Optimizer initialization
//! - Forward and backward passes
//! - Metric tracking and reporting

use anyhow::Result;
use candle_core::{DType, Device, Tensor};
use qlora_rs::{
    training::{QLoraTrainer, QLoraTrainingConfig},
    QLoraConfig, QuantizedLinear,
};

fn main() -> Result<()> {
    println!("=== QLoRA Training Example ===\n");

    // Set up device
    let device = Device::Cpu;

    // Create training configuration
    let training_config = QLoraTrainingConfig {
        use_paged_optimizer: false, // Use standard AdamW for CPU
        ..Default::default()
    };

    println!("Training Configuration:");
    println!(
        "  Learning rate: {}",
        training_config.adapter_config.learning_rate
    );
    println!(
        "  Weight decay: {}",
        training_config.adapter_config.weight_decay
    );
    println!(
        "  Gradient accumulation steps: {}",
        training_config.adapter_config.gradient_accumulation_steps
    );
    println!(
        "  Max gradient norm: {:?}",
        training_config.adapter_config.max_grad_norm
    );
    println!(
        "  Paged optimizer: {}\n",
        training_config.use_paged_optimizer
    );

    // Create QLoRA trainer
    let mut trainer = QLoraTrainer::new(training_config.clone(), device.clone());
    println!("QLoRA trainer created\n");

    // Create QLoRA configuration for the layer
    let lora_rank = 8;
    let lora_alpha = 16;
    let qlora_config = QLoraConfig::preset_qv_bf16(lora_rank, lora_alpha);

    println!("QLoRA Layer Configuration:");
    println!("  LoRA rank: {}", lora_rank);
    println!("  LoRA alpha: {}", lora_alpha);
    println!(
        "  Quantization block size: {}\n",
        qlora_config.quantization.block_size
    );

    // Define layer dimensions
    let in_features = 256;
    let out_features = 256;

    // Create a random weight tensor (simulating pre-trained model weights)
    println!(
        "Initializing layer with dimensions {}x{}...",
        out_features, in_features
    );
    let weight = Tensor::randn(0.0f32, 1.0f32, (out_features, in_features), &device)?;

    // IMPORTANT: Create layer using trainer's VarBuilder for gradient tracking
    // This registers the LoRA parameters in the trainer's VarMap
    let layer = {
        let vb = trainer.var_builder();
        QuantizedLinear::from_weight_with_varbuilder(&weight, None, &qlora_config, vb.pp("layer0"))?
    };
    println!("Layer created with VarBuilder (parameters registered for training)\n");

    // Initialize optimizer with the layer
    println!("Initializing optimizer...");
    trainer.init_optimizer(&[&layer])?;
    println!("Optimizer initialized with LoRA parameters\n");

    // Create mock training data
    let batch_size = 8;
    let seq_len = 64;

    println!("Preparing mock training data:");
    println!("  Batch size: {}", batch_size);
    println!("  Sequence length: {}", seq_len);

    // Mock input embeddings
    let input = Tensor::randn(0.0f32, 1.0f32, (batch_size, seq_len, in_features), &device)?;

    // Mock target labels (for demonstration - in real training, these would be actual labels)
    // Create random integers in range [0, out_features) by using randn and converting
    let targets_f32 = Tensor::randn(0.0f32, out_features as f32, (batch_size, seq_len), &device)?;
    let targets = targets_f32.abs()?.to_dtype(DType::U32)?;

    println!("  Input shape: {:?}", input.shape());
    println!("  Target shape: {:?}\n", targets.shape());

    // Training loop demonstration
    let num_steps = 5;
    println!("Running {} training steps...\n", num_steps);

    for step in 0..num_steps {
        println!("--- Step {} ---", step + 1);

        // Forward pass
        let output = layer.forward(&input)?;
        println!(
            "  Forward pass: {:?} -> {:?}",
            input.shape(),
            output.shape()
        );

        // Reshape output for loss calculation
        // From (batch, seq, features) to (batch * seq, features)
        let output_flat = output.reshape((batch_size * seq_len, out_features))?;
        let _targets_flat = targets.reshape((batch_size * seq_len,))?;

        // Compute mock loss (simple mean squared error for demonstration)
        // In real training, you'd use cross_entropy_loss for language modeling
        let loss = {
            // For simplicity, just use mean of squares as a proxy loss
            output_flat.sqr()?.mean_all()?
        };

        let loss_value = loss.to_scalar::<f32>()?;
        println!("  Loss: {:.6}", loss_value);

        // Backward pass
        // In a real training loop, you would:
        // 1. Compute gradients: loss.backward()
        // 2. Apply gradients with trainer
        // For this example, we just demonstrate the structure
        println!("  Backward pass: computing gradients (skipped in example)");
        println!("  Optimizer step: updating LoRA weights (skipped in example)");

        // Report metrics
        println!("  Metrics:");
        println!("    - Loss: {:.6}", loss_value);
        println!(
            "    - Learning rate: {}",
            training_config.adapter_config.learning_rate
        );

        // Check LoRA weight changes (in real training, these would update)
        let (lora_a, lora_b) = layer.lora_weights();
        let a_norm = lora_a.sqr()?.sum_all()?.to_scalar::<f32>()?.sqrt();
        let b_norm = lora_b.sqr()?.sum_all()?.to_scalar::<f32>()?.sqrt();
        println!("    - LoRA A norm: {:.6}", a_norm);
        println!("    - LoRA B norm: {:.6} (should be 0 initially)", b_norm);
        println!();
    }

    println!("=== Training Summary ===");
    println!("Successfully demonstrated QLoRA training setup:");
    println!("  ✓ Trainer initialized with configuration");
    println!("  ✓ Layer created with VarBuilder for gradient tracking");
    println!("  ✓ Optimizer initialized with LoRA parameters");
    println!("  ✓ Forward pass executed successfully");
    println!("  ✓ Loss computed on output");
    println!("\nNext steps for real training:");
    println!("  1. Load actual pre-trained model weights");
    println!("  2. Prepare real training dataset with DataLoader");
    println!("  3. Implement backward pass with loss.backward()");
    println!("  4. Call trainer.training_step() or trainer.training_step_lm()");
    println!("  5. Save checkpoints periodically");
    println!("  6. Monitor validation metrics");
    println!("\nKey benefits of QLoRA:");
    println!("  - Train large models on limited memory (4-bit base weights)");
    println!("  - Only update small LoRA adapters (~0.1% of parameters)");
    println!("  - Achieve near full fine-tuning quality");
    println!("  - Export to GGUF for efficient inference");

    Ok(())
}
