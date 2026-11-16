// MLX Kernel extracted from:
// https://github.com/ml-explore/mlx/blob/main/mlx/backend/metal/kernels/steel/gemm
// Copyright © 2024 Apple Inc.

#include <metal_simdgroup>
#include <metal_simdgroup_matrix>
#include <metal_stdlib>

#define STEEL_CONST static constant constexpr const
#define STEEL_PRAGMA_UNROLL _Pragma("clang loop unroll(full)")

using namespace metal;

// https://github.com/ml-explore/mlx/blob/02efb310cac667bc547d1b96f21596c221f84fe7/mlx/backend/metal/kernels/steel/gemm/params.h#L1
///////////////////////////////////////////////////////////////////////////////
// GEMM param classes
///////////////////////////////////////////////////////////////////////////////

struct GEMMParams {
  const int M;
  const int N;
  const int K;

  const int lda;
  const int ldb;
  const int ldd;

  const int tiles_n;
  const int tiles_m;

  const size_t batch_stride_a;
  const size_t batch_stride_b;
  const size_t batch_stride_d;

  const int swizzle_log;
  const int gemm_k_iterations_aligned;

  const int batch_ndim;
};

struct GEMMSpiltKParams {
  const int M;
  const int N;
  const int K;

  const int lda;
  const int ldb;
  const int ldc;

  const int tiles_n;
  const int tiles_m;

  const int split_k_partitions;
  const int split_k_partition_stride;
  const int split_k_partition_size;

  const int gemm_k_iterations_aligned;
};

struct GEMMAddMMParams {
  const int ldc;
  const int fdc;

  const size_t batch_stride_c;

  const float alpha;
  const float beta;
};

// https://github.com/ml-explore/mlx/blob/02efb310cac667bc547d1b96f21596c221f84fe7/mlx/backend/metal/kernels/steel/gemm/loader.h#L1
///////////////////////////////////////////////////////////////////////////////
// Loading helper
///////////////////////////////////////////////////////////////////////////////

template <
    typename T,
    short BROWS,
    short BCOLS,
    short dst_ld,
    short reduction_dim,
    short tgp_size,
    short alignment = 1,
    short n_reads = (BCOLS * BROWS) / (tgp_size),
    short TCOLS = BCOLS / n_reads,
    short TROWS = tgp_size / TCOLS>
struct BlockLoader {
  STEEL_CONST short n_rows = (BROWS + TROWS - 1) / TROWS;
  STEEL_CONST short vec_size = n_reads;

  // Leading dimension for src
  const int src_ld;
  const int tile_stride;

  // Thread location indices
  const short thread_idx;
  const short bi;
  const short bj;

  // threadgroup and device memory
  threadgroup T* dst;
  const device T* src;

  struct alignas(alignment * sizeof(T)) ReadVector {
    uint8_t v[sizeof(T) * vec_size];
  };

  /* Constructor */
  METAL_FUNC BlockLoader(
      const device T* src_,
      const int src_ld_,
      threadgroup T* dst_,
      ushort simd_group_id [[simdgroup_index_in_threadgroup]],
      ushort simd_lane_id [[thread_index_in_simdgroup]])
      : src_ld(src_ld_),
        tile_stride(reduction_dim ? BCOLS : BROWS * src_ld),
        thread_idx(simd_group_id * 32 + simd_lane_id),
        bi(thread_idx / TCOLS),
        bj(vec_size * (thread_idx % TCOLS)),
        dst(dst_ + bi * dst_ld + bj),
        src(src_ + bi * src_ld + bj) {}

  /* Apply operation to threadgroup without bound checking */
  template <typename UnaryOp>
  METAL_FUNC void apply_inplace_op(thread const UnaryOp& op) const {
    STEEL_PRAGMA_UNROLL
    for (short i = 0; i < BROWS; i += TROWS) {
      STEEL_PRAGMA_UNROLL
      for (short j = 0; j < vec_size; j++) {
        dst[i * dst_ld + j] = op.apply(dst[i * dst_ld + j]);
      }
    }
  }

  /* Load from device memory into threadgroup memory - without bound checking */
  METAL_FUNC void load_unsafe() const {
    STEEL_PRAGMA_UNROLL
    for (short i = 0; i < BROWS; i += TROWS) {
      *((threadgroup ReadVector*)(&dst[i * dst_ld])) =
          *((const device ReadVector*)(&src[i * src_ld]));
    }
  }

  /* Load from device memory into threadgroup memory - with bound checking */
  METAL_FUNC void load_safe(short2 src_tile_dim) const {
    src_tile_dim = src_tile_dim - short2(bj, bi);

    // Skip loading if thread has no valid reads
    if (src_tile_dim.x <= 0 || src_tile_dim.y <= 0) {
      STEEL_PRAGMA_UNROLL
      for (short i = 0; i < BROWS; i += TROWS) {
        STEEL_PRAGMA_UNROLL
        for (short j = 0; j < vec_size; j++) {
          dst[i * dst_ld + j] = T(0);
        }
      }
      return;
    }

    // Use fast thread memory for bound checks
    bool tmp_idx[vec_size];
    T tmp_val[vec_size];

    STEEL_PRAGMA_UNROLL
    for (short i = 0; i < BROWS; i += TROWS) {
      // Make sure tmp_idx only contains valid indices
      STEEL_PRAGMA_UNROLL
      for (short j = 0; j < vec_size; j++) {
        tmp_idx[j] = (i < src_tile_dim.y) && (j < src_tile_dim.x);
      }

      // Read valid indices into tmp_val
      STEEL_PRAGMA_UNROLL
      for (short j = 0; j < vec_size; j++) {
        tmp_val[j] = src[(tmp_idx[j] ? i * src_ld + j : 0)];
      }

      // Zero out unneeded values
      STEEL_PRAGMA_UNROLL
      for (short j = 0; j < vec_size; j++) {
        tmp_val[j] = tmp_idx[j] ? tmp_val[j] : T(0);
      }

      // Copy values to threadgroup memory
      STEEL_PRAGMA_UNROLL
      for (short j = 0; j < vec_size; j++) {
        dst[i * dst_ld + j] = tmp_val[j];
      }
    }
  }

  /* Iteration helper */
  METAL_FUNC void next() {
    src += tile_stride;
  }
};

// https://github.com/ml-explore/mlx/blob/02efb310cac667bc547d1b96f21596c221f84fe7/mlx/backend/metal/kernels/steel/gemm/transforms.h#L1
///////////////////////////////////////////////////////////////////////////////
// Transforms and Epilogues
///////////////////////////////////////////////////////////////////////////////

template <typename OutT, typename InT>
struct TransformNone {
  static METAL_FUNC OutT apply(InT x) {
    return static_cast<OutT>(x);
  }

  static METAL_FUNC OutT apply(InT x, OutT) {
    return static_cast<OutT>(x);
  }
};

template <typename OutT, typename InT>
struct TransformAdd {
  TransformAdd(const float, const float) {}

  static METAL_FUNC OutT apply(InT x) {
    return static_cast<OutT>(x);
  }

  static METAL_FUNC OutT apply(InT x, OutT c) {
    return static_cast<OutT>(x) + c;
  }
};

template <typename OutT, typename InT>
struct TransformAxpby {
  const float alpha;
  const float beta;

  TransformAxpby(const float alpha_, const float beta_)
      : alpha(alpha_), beta(beta_) {}

  static METAL_FUNC OutT apply(InT x) {
    return static_cast<OutT>(x);
  }

  METAL_FUNC OutT apply(InT x, OutT c) const {
    return static_cast<OutT>(x * alpha + (beta * c));
  }
};

template <typename T>
struct AccumHelper {
  typedef float accum_type;
};

struct BlockSwizzle {
  static METAL_FUNC int2
  swizzle(uint3 tid [[threadgroup_position_in_grid]], const int swizzle_log) {
    const int tid_x = (tid.x) >> swizzle_log;
    const int tid_y =
        ((tid.y) << swizzle_log) + ((tid.x) & ((1 << swizzle_log) - 1));
    return int2(tid_x, tid_y);
  }
};

// https://github.com/ml-explore/mlx/blob/02efb310cac667bc547d1b96f21596c221f84fe7/mlx/backend/metal/kernels/steel/gemm/mma.h#L1
///////////////////////////////////////////////////////////////////////////////
// MMA helper
///////////////////////////////////////////////////////////////////////////////

template <
    typename T,
    typename U,
    int BM,
    int BN,
    int BK,
    int WM,
    int WN,
    bool transpose_a,
    bool transpose_b,
    short lda_tgp,
    short ldb_tgp,
    typename AccumType = float,
    typename Epilogue = TransformNone<U, AccumType>>
struct BlockMMA {
  // Warp tile simdgroup matrix strides along M
  STEEL_CONST short TM_stride = 8 * WM;
  // Warp tile simdgroup matrix strides along M
  STEEL_CONST short TN_stride = 8 * WN;

  // Warp tile size along M
  STEEL_CONST short TM = BM / TM_stride;
  // Warp tile size along N
  STEEL_CONST short TN = BN / TN_stride;

  // Strides of A, B along reduction axis
  STEEL_CONST short simd_stride_a = {
      transpose_a ? TM_stride : TM_stride * lda_tgp};
  STEEL_CONST short simd_stride_b = {
      transpose_b ? TN_stride * ldb_tgp : TN_stride};

  // Jump between elements
  STEEL_CONST short jump_a = {transpose_a ? lda_tgp : 1};
  STEEL_CONST short jump_b = {transpose_b ? ldb_tgp : 1};

  STEEL_CONST short tile_stride_a = {transpose_a ? 8 * lda_tgp : 8};
  STEEL_CONST short tile_stride_b = {transpose_b ? 8 : 8 * ldb_tgp};

  // Simdgroup matrices
  simdgroup_matrix<AccumType, 8, 8> Asimd[TM];
  simdgroup_matrix<AccumType, 8, 8> Bsimd[TN];
  simdgroup_matrix<AccumType, 8, 8> results[TM * TN] = {
      simdgroup_matrix<AccumType, 8, 8>(0)};

  // Offsets within threadgroup
  const short tm;
  const short tn;

  short sm;
  short sn;

  short As_offset;
  short Bs_offset;

  /* Constructor */
  METAL_FUNC BlockMMA(
      ushort simd_group_id [[simdgroup_index_in_threadgroup]],
      ushort simd_lane_id [[thread_index_in_simdgroup]])
      : tm(8 * (simd_group_id / WN)), tn(8 * (simd_group_id % WN)) {
    // Determine thread position in simdgroup matrix
    short qid = simd_lane_id / 4;
    sm = (qid & 4) + (simd_lane_id / 2) % 4;
    sn = (qid & 2) * 2 + (simd_lane_id % 2) * 2;

    // Determine thread and simdgroup offset
    As_offset =
        transpose_a ? ((sn)*lda_tgp + (tm + sm)) : ((sn) + (tm + sm) * lda_tgp);
    Bs_offset =
        transpose_b ? ((tn + sn) * ldb_tgp + (sm)) : ((sm)*ldb_tgp + (tn + sn));
  }

  /* (BM, BK) X (BK, BN) multiply accumulate function */
  METAL_FUNC void mma(const threadgroup T* As, const threadgroup T* Bs) {
    // Adjust for simdgroup and thread location
    As += As_offset;
    Bs += Bs_offset;

    // Iterate over BK in blocks of 8
    STEEL_PRAGMA_UNROLL
    for (short kk = 0; kk < BK; kk += 8) {
      simdgroup_barrier(mem_flags::mem_none);

      // Load elements from threadgroup A as simdgroup matrices
      STEEL_PRAGMA_UNROLL
      for (short i = 0; i < TM; i++) {
        Asimd[i].thread_elements()[0] =
            static_cast<AccumType>(As[i * simd_stride_a + 0]);
        Asimd[i].thread_elements()[1] =
            static_cast<AccumType>(As[i * simd_stride_a + jump_a]);
      }

      simdgroup_barrier(mem_flags::mem_none);

      // Load elements from threadgroup B as simdgroup matrices
      STEEL_PRAGMA_UNROLL
      for (short j = 0; j < TN; j++) {
        Bsimd[j].thread_elements()[0] =
            static_cast<AccumType>(Bs[j * simd_stride_b + 0]);
        Bsimd[j].thread_elements()[1] =
            static_cast<AccumType>(Bs[j * simd_stride_b + jump_b]);
      }

      simdgroup_barrier(mem_flags::mem_none);

      // Multiply and accumulate into result simdgroup matrices
      STEEL_PRAGMA_UNROLL
      for (short i = 0; i < TM; i++) {
        STEEL_PRAGMA_UNROLL
        for (short j = 0; j < TN; j++) {
          short j_serp = (i % 2) ? (TN - 1 - j) : j;

          simdgroup_multiply_accumulate(
              results[i * TN + j_serp],
              Asimd[i],
              Bsimd[j_serp],
              results[i * TN + j_serp]);
        }
      }

      // Progress to next simdgroup tile
      As += tile_stride_a;
      Bs += tile_stride_b;
    }
  }

  /* Store results from simdgroup_matrix results into device memory */
  METAL_FUNC void store_result(device U* D, const int ldd) const {
    // Adjust for simdgroup and thread location
    D += (sm + tm) * ldd + tn + sn;

    // Loop over all simdgroup tiles
    STEEL_PRAGMA_UNROLL
    for (short i = 0; i < TM; i++) {
      STEEL_PRAGMA_UNROLL
      for (short j = 0; j < TN; j++) {
        // Get accumulated result and associated offset in C
        thread const auto& accum = results[i * TN + j].thread_elements();
        int offset = (i * TM_stride) * ldd + (j * TN_stride);

        // Apply epilogue
        U outs[2] = {Epilogue::apply(accum[0]), Epilogue::apply(accum[1])};

        // Write out D
        D[offset] = outs[0];
        D[offset + 1] = outs[1];
      }
    }
  }

  METAL_FUNC void
  store_result_safe(device U* D, const int ldd, short2 dst_tile_dims) const {
    // Adjust for simdgroup and thread location
    D += (sm + tm) * ldd + (tn + sn);
    dst_tile_dims -= short2(tn + sn, sm + tm);

    if (dst_tile_dims.x <= 0 || dst_tile_dims.y <= 0)
      return;

    STEEL_PRAGMA_UNROLL
    for (int i = 0; i < TM; i++) {
      if (i * TM_stride < dst_tile_dims.y) {
        STEEL_PRAGMA_UNROLL
        for (int j = 0; j < TN; j++) {
          // Get accumulated result and associated offset in C
          thread const auto& accum = results[i * TN + j].thread_elements();
          int offset = (i * TM_stride) * ldd + (j * TN_stride);

          // Apply epilogue and output C
          if (j * TN_stride < dst_tile_dims.x) {
            D[offset] = Epilogue::apply(accum[0]);
          }

          if (j * TN_stride + 1 < dst_tile_dims.x) {
            D[offset + 1] = Epilogue::apply(accum[1]);
          }
        }
      }
    }
  }

  /* Apply epilogue */
  template <typename UnaryEpilogue>
  METAL_FUNC void apply_epilogue(thread const UnaryEpilogue& epilogue_op) {
    // Loop over all simdgroup tiles
    STEEL_PRAGMA_UNROLL
    for (short i = 0; i < TM; i++) {
      STEEL_PRAGMA_UNROLL
      for (short j = 0; j < TN; j++) {
        // Get accumulated result and associated offset in C
        thread auto& accum = results[i * TN + j].thread_elements();

        // Apply epilogue
        accum[0] = epilogue_op.apply(accum[0]);
        accum[1] = epilogue_op.apply(accum[1]);
      }
    }
  }

  /* Apply epilogue */
  template <typename BinaryEpilogue>
  METAL_FUNC void apply_epilogue(
      const device U* C,
      const int ldc,
      const int fdc,
      thread const BinaryEpilogue& epilogue_op) {
    // Adjust for simdgroup and thread location
    C += (sm + tm) * ldc + (tn + sn) * fdc;

    // Loop over all simdgroup tiles
    STEEL_PRAGMA_UNROLL
    for (short i = 0; i < TM; i++) {
      STEEL_PRAGMA_UNROLL
      for (short j = 0; j < TN; j++) {
        // Get accumulated result and associated offset in C
        thread auto& accum = results[i * TN + j].thread_elements();
        int offset_c = (i * TM_stride) * ldc + (j * TN_stride) * fdc;

        // Apply epilogue
        accum[0] = epilogue_op.apply(accum[0], C[offset_c]);
        accum[1] = epilogue_op.apply(accum[1], C[offset_c + fdc]);
      }
    }
  }

  /* Apply epilogue */
  template <typename BinaryEpilogue>
  METAL_FUNC void apply_epilogue_safe(
      const device U* C,
      const int ldc,
      const int fdc,
      short2 dst_tile_dims,
      thread const BinaryEpilogue& epilogue_op) {
    // Adjust for simdgroup and thread location
    C += (sm + tm) * ldc + (tn + sn) * fdc;
    dst_tile_dims -= short2(tn + sn, sm + tm);

    if (dst_tile_dims.x <= 0 || dst_tile_dims.y <= 0)
      return;

    // Loop over all simdgroup tiles
    STEEL_PRAGMA_UNROLL
    for (short i = 0; i < TM; i++) {
      STEEL_PRAGMA_UNROLL
      for (short j = 0; j < TN; j++) {
        // Get accumulated result and associated offset in C
        thread auto& accum = results[i * TN + j].thread_elements();
        int offset_c = (i * TM_stride) * ldc + (j * TN_stride) * fdc;

        // Read C
        U c_elems[2] = {0};

        if ((j * TN_stride + 1) < dst_tile_dims.x) {
          c_elems[0] = C[offset_c];
          c_elems[1] = C[offset_c + fdc];
        } else if ((j * TN_stride) < dst_tile_dims.x) {
          c_elems[0] = C[offset_c];
        }

        // Apply epilogue
        accum[0] = epilogue_op.apply(accum[0], c_elems[0]);
        accum[1] = epilogue_op.apply(accum[1], c_elems[1]);
      }
    }
  }

  /* Store results from simdgroup_matrix results into device memory */
  METAL_FUNC void store_result(
      device U* D,
      const int ldd,
      const device U* C,
      const int ldc,
      const int fdc,
      thread const Epilogue& epilogue_op) const {
    // Adjust for simdgroup and thread location
    C += (sm + tm) * ldc + (tn + sn) * fdc;
    D += (sm + tm) * ldd + tn + sn;

    // Loop over all simdgroup tiles
    STEEL_PRAGMA_UNROLL
    for (short i = 0; i < TM; i++) {
      STEEL_PRAGMA_UNROLL
      for (short j = 0; j < TN; j++) {
        // Get accumulated result and associated offset in C
        thread const auto& accum = results[i * TN + j].thread_elements();
        int offset_c = (i * TM_stride) * ldc + (j * TN_stride) * fdc;
        int offset_d = (i * TM_stride) * ldd + (j * TN_stride);

        // Apply epilogue
        U outs[2] = {
            epilogue_op.apply(accum[0], C[offset_c]),
            epilogue_op.apply(accum[1], C[offset_c + fdc])};

        // Write out D
        D[offset_d] = outs[0];
        D[offset_d + 1] = outs[1];
      }
    }
  }

  METAL_FUNC void store_result_safe(
      device U* D,
      const int ldd,
      const device U* C,
      const int ldc,
      const int fdc,
      short2 dst_tile_dims,
      thread const Epilogue& epilogue_op) const {
    // Adjust for simdgroup and thread location
    C += (sm + tm) * ldc + (tn + sn) * fdc;
    D += (sm + tm) * ldd + tn + sn;
    dst_tile_dims -= short2(tn + sn, sm + tm);

    if (dst_tile_dims.x <= 0 || dst_tile_dims.y <= 0)
      return;

    STEEL_PRAGMA_UNROLL
    for (int i = 0; i < TM; i++) {
      if (i * TM_stride < dst_tile_dims.y) {
        STEEL_PRAGMA_UNROLL
        for (int j = 0; j < TN; j++) {
          // Get accumulated result and associated offset in C
          thread const auto& accum = results[i * TN + j].thread_elements();
          int offset_c = (i * TM_stride) * ldc + (j * TN_stride) * fdc;
          int offset_d = (i * TM_stride) * ldd + (j * TN_stride);

          // Apply epilogue and output C
          if (j * TN_stride < dst_tile_dims.x) {
            D[offset_d] = epilogue_op.apply(accum[0], C[offset_c]);
          }

          if (j * TN_stride + 1 < dst_tile_dims.x) {
            D[offset_d + 1] = epilogue_op.apply(accum[1], C[offset_c + fdc]);
          }
        }
      }
    }
  }
};

// https://github.com/ml-explore/mlx/blob/02efb310cac667bc547d1b96f21596c221f84fe7/mlx/backend/metal/kernels/steel/gemm/gemm.h#L1
///////////////////////////////////////////////////////////////////////////////
// GEMM kernel class
///////////////////////////////////////////////////////////////////////////////

template <bool M_aligned, bool N_aligned, bool K_aligned>
struct LoopAlignment {};

template <
    typename T,
    typename U,
    int BM,
    int BN,
    int BK,
    int WM,
    int WN,
    bool transpose_a,
    bool transpose_b,
    bool MN_aligned,
    bool K_aligned,
    typename AccumType = typename AccumHelper<T>::accum_type,
    typename Epilogue = TransformNone<U, AccumType>>
struct GEMMKernel {
  STEEL_CONST short tgp_padding_a = 16 / sizeof(T);
  STEEL_CONST short tgp_padding_b = 16 / sizeof(T);
  STEEL_CONST short tgp_mem_size_a =
      transpose_a ? BK * (BM + tgp_padding_a) : BM * (BK + tgp_padding_a);
  STEEL_CONST short tgp_mem_size_b =
      transpose_b ? BN * (BK + tgp_padding_b) : BK * (BN + tgp_padding_b);
  STEEL_CONST short tgp_mem_size = tgp_mem_size_a + tgp_mem_size_b;

  STEEL_CONST short tgp_size = WM * WN * 32;

  using loader_a_t = BlockLoader<
      T,
      transpose_a ? BK : BM,
      transpose_a ? BM : BK,
      transpose_a ? BM + tgp_padding_a : BK + tgp_padding_a,
      !transpose_a,
      tgp_size>;
  using loader_b_t = BlockLoader<
      T,
      transpose_b ? BN : BK,
      transpose_b ? BK : BN,
      transpose_b ? BK + tgp_padding_b : BN + tgp_padding_b,
      transpose_b,
      tgp_size>;
  using mma_t = BlockMMA<
      T,
      U,
      BM,
      BN,
      BK,
      WM,
      WN,
      transpose_a,
      transpose_b,
      transpose_a ? BM + tgp_padding_a : BK + tgp_padding_a,
      transpose_b ? BK + tgp_padding_b : BN + tgp_padding_b,
      AccumType,
      Epilogue>;

  /* Main kernel function */
  template <bool M_aligned, bool N_aligned, bool K_aligned_>
  static METAL_FUNC void gemm_loop(
      threadgroup T* As [[threadgroup(0)]],
      threadgroup T* Bs [[threadgroup(1)]],
      const int gemm_k_iterations,
      thread loader_a_t& loader_a,
      thread loader_b_t& loader_b,
      thread mma_t& mma_op,
      thread const short& tgp_bm,
      thread const short& tgp_bn,
      thread const short& lbk,
      LoopAlignment<M_aligned, N_aligned, K_aligned_> l = {}) {
    // Appease the compiler
    (void)l;

    short2 tile_dims_A = transpose_a ? short2(tgp_bm, BK) : short2(BK, tgp_bm);

    short2 tile_dims_B = transpose_b ? short2(BK, tgp_bn) : short2(tgp_bn, BK);

    for (int k = 0; k < gemm_k_iterations; k++) {
      threadgroup_barrier(mem_flags::mem_threadgroup);
      // Load elements into threadgroup
      if (M_aligned) {
        loader_a.load_unsafe();
      } else {
        loader_a.load_safe(tile_dims_A);
      }

      if (N_aligned) {
        loader_b.load_unsafe();
      } else {
        loader_b.load_safe(tile_dims_B);
      }

      threadgroup_barrier(mem_flags::mem_threadgroup);

      // Multiply and accumulate threadgroup elements
      mma_op.mma(As, Bs);

      // Prepare for next iteration
      loader_a.next();
      loader_b.next();
    }

    if (!K_aligned_) {
      threadgroup_barrier(mem_flags::mem_threadgroup);

      short2 tile_dims_A_last =
          transpose_a ? short2(tgp_bm, lbk) : short2(lbk, tgp_bm);
      short2 tile_dims_B_last =
          transpose_b ? short2(lbk, tgp_bn) : short2(tgp_bn, lbk);

      loader_a.load_safe(tile_dims_A_last);
      loader_b.load_safe(tile_dims_B_last);

      threadgroup_barrier(mem_flags::mem_threadgroup);

      mma_op.mma(As, Bs);
    }
  }

  /* Main kernel function */
  static METAL_FUNC void run(
      const device T* A [[buffer(0)]],
      const device T* B [[buffer(1)]],
      device U* D [[buffer(2)]],
      const constant GEMMParams* params [[buffer(3)]],
      threadgroup T* As [[threadgroup(0)]],
      threadgroup T* Bs [[threadgroup(1)]],
      uint simd_lane_id [[thread_index_in_simdgroup]],
      uint simd_group_id [[simdgroup_index_in_threadgroup]],
      uint3 tid [[threadgroup_position_in_grid]],
      uint3 lid [[thread_position_in_threadgroup]]) {
    // Pacifying compiler
    (void)lid;

    const int tid_y = ((tid.y) << params->swizzle_log) +
        ((tid.x) & ((1 << params->swizzle_log) - 1));
    const int tid_x = (tid.x) >> params->swizzle_log;

    if (params->tiles_n <= tid_x || params->tiles_m <= tid_y) {
      return;
    }

    threadgroup_barrier(mem_flags::mem_none);

    // Find block in A, B, C
    const int c_row = tid_y * BM;
    const int c_col = tid_x * BN;
    const size_t c_row_long = size_t(c_row);
    const size_t c_col_long = size_t(c_col);

    A += transpose_a ? c_row_long : c_row_long * params->lda;
    B += transpose_b ? c_col_long * params->ldb : c_col_long;
    D += c_row_long * params->ldd + c_col_long;

    // Prepare threadgroup loading operations
    thread loader_a_t loader_a(A, params->lda, As, simd_group_id, simd_lane_id);
    thread loader_b_t loader_b(B, params->ldb, Bs, simd_group_id, simd_lane_id);

    // Prepare threadgroup mma operation
    thread mma_t mma_op(simd_group_id, simd_lane_id);

    int gemm_k_iterations = params->gemm_k_iterations_aligned;

    ///////////////////////////////////////////////////////////////////////////////
    // MNK aligned loop
    if (MN_aligned) {
      for (int k = 0; k < gemm_k_iterations; k++) {
        threadgroup_barrier(mem_flags::mem_threadgroup);
        // Load elements into threadgroup
        loader_a.load_unsafe();
        loader_b.load_unsafe();

        threadgroup_barrier(mem_flags::mem_threadgroup);

        // Multiply and accumulate threadgroup elements
        mma_op.mma(As, Bs);

        // Prepare for next iteration
        loader_a.next();
        loader_b.next();
      }

      threadgroup_barrier(mem_flags::mem_none);

      // Loop tail
      if (!K_aligned) {
        int lbk = params->K - params->gemm_k_iterations_aligned * BK;
        short2 tile_dims_A = transpose_a ? short2(BM, lbk) : short2(lbk, BM);
        short2 tile_dims_B = transpose_b ? short2(lbk, BN) : short2(BN, lbk);

        loader_a.load_safe(tile_dims_A);
        loader_b.load_safe(tile_dims_B);

        threadgroup_barrier(mem_flags::mem_threadgroup);

        mma_op.mma(As, Bs);
      }

      // Store results to device memory
      mma_op.store_result(D, params->ldd);
      return;

    }
    ///////////////////////////////////////////////////////////////////////////////
    // MN unaligned loop
    else { // Loop over K - unaligned case
      short tgp_bm = min(BM, params->M - c_row);
      short tgp_bn = min(BN, params->N - c_col);
      short leftover_bk = params->K - params->gemm_k_iterations_aligned * BK;

      if (tgp_bm == BM && tgp_bn == BN) {
        gemm_loop<true, true, K_aligned>(
            As,
            Bs,
            gemm_k_iterations,
            loader_a,
            loader_b,
            mma_op,
            tgp_bm,
            tgp_bn,
            leftover_bk);

        mma_op.store_result(D, params->ldd);
        return;

      } else if (tgp_bn == BN) {
        gemm_loop<false, true, K_aligned>(
            As,
            Bs,
            gemm_k_iterations,
            loader_a,
            loader_b,
            mma_op,
            tgp_bm,
            tgp_bn,
            leftover_bk);

        mma_op.store_result_safe(D, params->ldd, short2(tgp_bn, tgp_bm));
        return;

      } else if (tgp_bm == BM) {
        gemm_loop<true, false, K_aligned>(
            As,
            Bs,
            gemm_k_iterations,
            loader_a,
            loader_b,
            mma_op,
            tgp_bm,
            tgp_bn,
            leftover_bk);

        mma_op.store_result_safe(D, params->ldd, short2(tgp_bn, tgp_bm));
        return;

      } else {
        gemm_loop<false, false, K_aligned>(
            As,
            Bs,
            gemm_k_iterations,
            loader_a,
            loader_b,
            mma_op,
            tgp_bm,
            tgp_bn,
            leftover_bk);

        mma_op.store_result_safe(D, params->ldd, short2(tgp_bn, tgp_bm));
        return;
      }
    }
  }
};

// utils.h
///////////////////////////////////////////////////////////////////////////////
// Single Array with generic dims

template <typename stride_t>
METAL_FUNC stride_t elem_to_loc(
    uint elem,
    device const int* shape,
    device const stride_t* strides,
    int ndim) {
  stride_t loc = 0;
  for (int i = ndim - 1; i >= 0 && elem > 0; --i) {
    loc += (elem % shape[i]) * strides[i];
    elem /= shape[i];
  }
  return loc;
}

template <typename stride_t>
METAL_FUNC stride_t elem_to_loc(
    uint elem,
    constant const int* shape,
    constant const stride_t* strides,
    int ndim) {
  stride_t loc = 0;
  for (int i = ndim - 1; i >= 0 && elem > 0; --i) {
    loc += (elem % shape[i]) * strides[i];
    elem /= shape[i];
  }
  return loc;
}

template <typename stride_t>
METAL_FUNC stride_t elem_to_loc(
    stride_t elem,
    device const int* shape,
    device const stride_t* strides,
    int ndim) {
  stride_t loc = 0;
  for (int i = ndim - 1; i >= 0 && elem > 0; --i) {
    loc += (elem % shape[i]) * strides[i];
    elem /= shape[i];
  }
  return loc;
}

template <typename stride_t>
METAL_FUNC stride_t elem_to_loc(
    stride_t elem,
    constant const int* shape,
    constant const stride_t* strides,
    int ndim) {
  stride_t loc = 0;
  for (int i = ndim - 1; i >= 0 && elem > 0; --i) {
    loc += (elem % shape[i]) * strides[i];
    elem /= shape[i];
  }
  return loc;
}

// Non templated version to handle arbitrary dims
template <typename stride_t>
METAL_FUNC stride_t elem_to_loc(
    uint3 elem,
    constant const int* shape,
    constant const stride_t* strides,
    int ndim) {
  stride_t loc = elem.x * strides[ndim - 1] + elem.y * strides[ndim - 2];
  for (int d = ndim - 3; d >= 0; --d) {
    loc += (elem.z % shape[d]) * strides[d];
    elem.z /= shape[d];
  }
  return loc;
}


METAL_FUNC ulong2 elem_to_loc_broadcast(
    uint elem,
    constant const int* shape,
    constant const size_t* a_strides,
    constant const size_t* b_strides,
    int ndim) {
  ulong loc_a{0};
  ulong loc_b{0};
  for (int i = ndim - 1; i >= 0 && elem > 0; --i) {
    int pos_in_dim = (elem % shape[i]);
    elem /= shape[i];
    loc_a += pos_in_dim * a_strides[i];
    loc_b += pos_in_dim * b_strides[i];
  }
  return ulong2(loc_a, loc_b);
}

METAL_FUNC ulong3 elem_to_loc_broadcast(
    uint elem,
    constant const int* shape,
    constant const size_t* a_strides,
    constant const size_t* b_strides,
    constant const size_t* c_strides,
    int ndim) {
  ulong loc_a{0};
  ulong loc_b{0};
  ulong loc_c{0};
  for (int i = ndim - 1; i >= 0 && elem > 0; --i) {
    int pos_in_dim = (elem % shape[i]);
    elem /= shape[i];
    loc_a += pos_in_dim * a_strides[i];
    loc_b += pos_in_dim * b_strides[i];
    loc_c += pos_in_dim * c_strides[i];
  }
  return ulong3(loc_a, loc_b, loc_c);
}


// https://github.com/ml-explore/mlx/blob/02efb310cac667bc547d1b96f21596c221f84fe7/mlx/backend/metal/kernels/steel/gemm/kernels/steel_gemm_fused.h#L1
///////////////////////////////////////////////////////////////////////////////
// GEMM kernels
///////////////////////////////////////////////////////////////////////////////

constant bool has_batch [[function_constant(10)]];

constant bool use_out_source [[function_constant(100)]];
constant bool do_axpby [[function_constant(110)]];

constant bool align_M [[function_constant(200)]];
constant bool align_N [[function_constant(201)]];
constant bool align_K [[function_constant(202)]];

constant bool do_gather [[function_constant(300)]];

constant bool gather_bias = do_gather && use_out_source;

// clang-format off
template <
    typename T,
    int BM,
    int BN,
    int BK,
    int WM,
    int WN,
    bool transpose_a,
    bool transpose_b,
    typename AccumType = float>
[[kernel, max_total_threads_per_threadgroup(WM* WN * 32)]] void gemm(
    const device T* A [[buffer(0)]],
    const device T* B [[buffer(1)]],
    const device T* C [[buffer(2), function_constant(use_out_source)]],
    device T* D [[buffer(3)]],
    const constant GEMMParams* params [[buffer(4)]],
    const constant GEMMAddMMParams* addmm_params [[buffer(5), function_constant(use_out_source)]],
    const constant int* batch_shape [[buffer(6), function_constant(has_batch)]],
    const constant size_t* batch_strides [[buffer(7), function_constant(has_batch)]],
    const constant uint32_t* lhs_indices [[buffer(10), function_constant(do_gather)]],
    const constant uint32_t* rhs_indices [[buffer(11), function_constant(do_gather)]],
    const constant uint32_t* C_indices [[buffer(12), function_constant(gather_bias)]],
    const constant int* operand_shape [[buffer(13), function_constant(do_gather)]],
    const constant size_t* operand_strides [[buffer(14), function_constant(do_gather)]],
    const constant packed_int3& operand_batch_ndim [[buffer(15), function_constant(do_gather)]],
    uint simd_lane_id [[thread_index_in_simdgroup]],
    uint simd_group_id [[simdgroup_index_in_threadgroup]],
    uint3 tid [[threadgroup_position_in_grid]],
    uint3 lid [[thread_position_in_threadgroup]]) { // clang-format on
  // Pacifying compiler
  (void)lid;

  using gemm_kernel = GEMMKernel<
      T,
      T,
      BM,
      BN,
      BK,
      WM,
      WN,
      transpose_a,
      transpose_b,
      true,
      true,
      AccumType>;

  using loader_a_t = typename gemm_kernel::loader_a_t;
  using loader_b_t = typename gemm_kernel::loader_b_t;
  using mma_t = typename gemm_kernel::mma_t;

  // Find block
  const int tid_y = ((tid.y) << params->swizzle_log) +
      ((tid.x) & ((1 << params->swizzle_log) - 1));
  const int tid_x = (tid.x) >> params->swizzle_log;

  // Exit early if out of bounds
  if (params->tiles_n <= tid_x || params->tiles_m <= tid_y) {
    return;
  }

  // Adjust for batch

  // Handle gather
  if (do_gather) {
    // Read indices
    uint32_t indx_A, indx_B, indx_C;

    if (has_batch) {
      const constant size_t* indx_A_bstrides = batch_strides;
      const constant size_t* indx_B_bstrides =
          batch_strides + params->batch_ndim;

      ulong2 indx_offsets = elem_to_loc_broadcast(
          tid.z,
          batch_shape,
          indx_A_bstrides,
          indx_B_bstrides,
          params->batch_ndim);
      indx_A = lhs_indices[indx_offsets.x];
      indx_B = rhs_indices[indx_offsets.y];

      if (use_out_source) {
        const constant size_t* indx_C_bstrides =
            indx_B_bstrides + params->batch_ndim;
        auto indx_offset_C = elem_to_loc(
            tid.z, batch_shape, indx_C_bstrides, params->batch_ndim);
        indx_C = C_indices[indx_offset_C];
      }
    } else {
      indx_A = lhs_indices[params->batch_stride_a * tid.z];
      indx_B = rhs_indices[params->batch_stride_b * tid.z];

      if (use_out_source) {
        indx_C = C_indices[addmm_params->batch_stride_c * tid.z];
      }
    }

    // Translate indices to offsets
    int batch_ndim_A = operand_batch_ndim.x;
    const constant int* batch_shape_A = operand_shape;
    const constant size_t* batch_strides_A = operand_strides;
    A += elem_to_loc(indx_A, batch_shape_A, batch_strides_A, batch_ndim_A);

    int batch_ndim_B = operand_batch_ndim.y;
    const constant int* batch_shape_B = batch_shape_A + batch_ndim_A;
    const constant size_t* batch_strides_B = batch_strides_A + batch_ndim_A;
    B += elem_to_loc(indx_B, batch_shape_B, batch_strides_B, batch_ndim_B);

    if (use_out_source) {
      int batch_ndim_C = operand_batch_ndim.z;
      const constant int* batch_shape_C = batch_shape_B + batch_ndim_B;
      const constant size_t* batch_strides_C = batch_strides_B + batch_ndim_B;
      C += elem_to_loc(indx_C, batch_shape_C, batch_strides_C, batch_ndim_C);
    }

  }

  // Handle regular batch
  else {
    if (has_batch) {
      const constant size_t* A_bstrides = batch_strides;
      const constant size_t* B_bstrides = batch_strides + params->batch_ndim;

      ulong2 batch_offsets = elem_to_loc_broadcast(
          tid.z, batch_shape, A_bstrides, B_bstrides, params->batch_ndim);

      A += batch_offsets.x;
      B += batch_offsets.y;

      if (use_out_source) {
        const constant size_t* C_bstrides = B_bstrides + params->batch_ndim;
        C += elem_to_loc(tid.z, batch_shape, C_bstrides, params->batch_ndim);
      }
    } else {
      A += params->batch_stride_a * tid.z;
      B += params->batch_stride_b * tid.z;

      if (use_out_source) {
        C += addmm_params->batch_stride_c * tid.z;
      }
    }
  }

  D += params->batch_stride_d * tid.z;

  // Prepare threadgroup memory
  threadgroup T As[gemm_kernel::tgp_mem_size_a];
  threadgroup T Bs[gemm_kernel::tgp_mem_size_b];

  threadgroup_barrier(mem_flags::mem_none);

  // Find block in A, B, C
  const int c_row = tid_y * BM;
  const int c_col = tid_x * BN;
  const size_t c_row_long = size_t(c_row);
  const size_t c_col_long = size_t(c_col);

  A += transpose_a ? c_row_long : c_row_long * params->lda;
  B += transpose_b ? c_col_long * params->ldb : c_col_long;
  D += c_row_long * params->ldd + c_col_long;

  if (use_out_source) {
    C += c_row_long * addmm_params->ldc + c_col_long * addmm_params->fdc;
  }

  // Prepare threadgroup mma operation
  thread mma_t mma_op(simd_group_id, simd_lane_id);

  // Prepare threadgroup loading operations
  thread loader_a_t loader_a(A, params->lda, As, simd_group_id, simd_lane_id);
  thread loader_b_t loader_b(B, params->ldb, Bs, simd_group_id, simd_lane_id);

  // Prepare threadgroup bounds
  const short tgp_bm = align_M ? BM : short(min(BM, params->M - c_row));
  const short tgp_bn = align_N ? BN : short(min(BN, params->N - c_col));

  // Prepare iterations
  int gemm_k_iterations = params->gemm_k_iterations_aligned;

  // Do unaligned K iterations first
  if (!align_K) {
    const int k_last = params->gemm_k_iterations_aligned * BK;
    const int k_remain = params->K - k_last;
    const size_t k_jump_a =
        transpose_a ? params->lda * size_t(k_last) : size_t(k_last);
    const size_t k_jump_b =
        transpose_b ? size_t(k_last) : params->ldb * size_t(k_last);

    // Move loader source ahead to end
    loader_a.src += k_jump_a;
    loader_b.src += k_jump_b;

    // Load tile
    const short2 tile_dims_A =
        transpose_a ? short2(tgp_bm, k_remain) : short2(k_remain, tgp_bm);
    const short2 tile_dims_B =
        transpose_b ? short2(k_remain, tgp_bn) : short2(tgp_bn, k_remain);

    loader_a.load_safe(tile_dims_A);
    loader_b.load_safe(tile_dims_B);

    threadgroup_barrier(mem_flags::mem_threadgroup);

    // Do matmul
    mma_op.mma(As, Bs);

    // Reset source back to start
    loader_a.src -= k_jump_a;
    loader_b.src -= k_jump_b;
  }

  const TransformAdd<AccumType, AccumType> epilogue_op_add(
      addmm_params->alpha, addmm_params->beta);
  const TransformAxpby<AccumType, AccumType> epilogue_op_axpby(
      addmm_params->alpha, addmm_params->beta);

  ///////////////////////////////////////////////////////////////////////////////
  // MNK aligned loop
  if (align_M && align_N) {
    // Do gemm
    for (int k = 0; k < gemm_k_iterations; k++) {
      threadgroup_barrier(mem_flags::mem_threadgroup);
      // Load elements into threadgroup
      loader_a.load_unsafe();
      loader_b.load_unsafe();

      threadgroup_barrier(mem_flags::mem_threadgroup);

      // Multiply and accumulate threadgroup elements
      mma_op.mma(As, Bs);

      // Prepare for next iteration
      loader_a.next();
      loader_b.next();
    }

    threadgroup_barrier(mem_flags::mem_none);

    // Do epilogue
    if (use_out_source) {
      if (do_axpby) {
        mma_op.apply_epilogue(
            C, addmm_params->ldc, addmm_params->fdc, epilogue_op_axpby);
      } else {
        mma_op.apply_epilogue(
            C, addmm_params->ldc, addmm_params->fdc, epilogue_op_add);
      }
    }

    // Store results to device memory
    return mma_op.store_result(D, params->ldd);

  }
  ///////////////////////////////////////////////////////////////////////////////
  // MN unaligned loop
  else { // Loop over K - unaligned case
    const int leftover_bk = 0;

    if ((align_M || tgp_bm == BM) && (align_N || tgp_bn == BN)) {
      // Do gemm
      gemm_kernel::gemm_loop(
          As,
          Bs,
          gemm_k_iterations,
          loader_a,
          loader_b,
          mma_op,
          tgp_bm,
          tgp_bn,
          leftover_bk,
          LoopAlignment<true, true, true>{});

      // Do epilogue
      if (use_out_source) {
        if (do_axpby) {
          mma_op.apply_epilogue(
              C, addmm_params->ldc, addmm_params->fdc, epilogue_op_axpby);
        } else {
          mma_op.apply_epilogue(
              C, addmm_params->ldc, addmm_params->fdc, epilogue_op_add);
        }
      }

      // Store results to device memory
      return mma_op.store_result(D, params->ldd);

    } else if (align_N || tgp_bn == BN) {
      gemm_kernel::gemm_loop(
          As,
          Bs,
          gemm_k_iterations,
          loader_a,
          loader_b,
          mma_op,
          tgp_bm,
          tgp_bn,
          leftover_bk,
          LoopAlignment<false, true, true>{});

      // Do epilogue
      if (use_out_source) {
        if (do_axpby) {
          mma_op.apply_epilogue_safe(
              C,
              addmm_params->ldc,
              addmm_params->fdc,
              short2(tgp_bn, tgp_bm),
              epilogue_op_axpby);
        } else {
          mma_op.apply_epilogue_safe(
              C,
              addmm_params->ldc,
              addmm_params->fdc,
              short2(tgp_bn, tgp_bm),
              epilogue_op_add);
        }
      }

      // Store results to device memory
      return mma_op.store_result_safe(D, params->ldd, short2(tgp_bn, tgp_bm));

    } else if (align_M || tgp_bm == BM) {
      gemm_kernel::gemm_loop(
          As,
          Bs,
          gemm_k_iterations,
          loader_a,
          loader_b,
          mma_op,
          tgp_bm,
          tgp_bn,
          leftover_bk,
          LoopAlignment<true, false, true>{});

      // Do epilogue
      if (use_out_source) {
        if (do_axpby) {
          mma_op.apply_epilogue_safe(
              C,
              addmm_params->ldc,
              addmm_params->fdc,
              short2(tgp_bn, tgp_bm),
              epilogue_op_axpby);
        } else {
          mma_op.apply_epilogue_safe(
              C,
              addmm_params->ldc,
              addmm_params->fdc,
              short2(tgp_bn, tgp_bm),
              epilogue_op_add);
        }
      }

      // Store results to device memory
      return mma_op.store_result_safe(D, params->ldd, short2(tgp_bn, tgp_bm));

    } else {
      gemm_kernel::gemm_loop(
          As,
          Bs,
          gemm_k_iterations,
          loader_a,
          loader_b,
          mma_op,
          tgp_bm,
          tgp_bn,
          leftover_bk,
          LoopAlignment<false, false, true>{});

      // Do epilogue
      if (use_out_source) {
        if (do_axpby) {
          mma_op.apply_epilogue_safe(
              C,
              addmm_params->ldc,
              addmm_params->fdc,
              short2(tgp_bn, tgp_bm),
              epilogue_op_axpby);
        } else {
          mma_op.apply_epilogue_safe(
              C,
              addmm_params->ldc,
              addmm_params->fdc,
              short2(tgp_bn, tgp_bm),
              epilogue_op_add);
        }
      }

      // Store results to device memory
      return mma_op.store_result_safe(D, params->ldd, short2(tgp_bn, tgp_bm));
    }
  }
}

#define instantiate_gemm(tname, trans_a, trans_b, iname, itype, oname, otype, bm, bn, bk, wm, wn) \
  template [[host_name("gemm_" #tname "_"  #iname "_" #oname "_" #bm "_" #bn "_" #bk "_" #wm "_" #wn)]] \
  [[kernel]] void gemm<itype, bm, bn, bk, wm, wn, trans_a, trans_b, float>( \
      const device itype *A [[buffer(0)]], \
      const device itype *B [[buffer(1)]], \
      const device itype *C [[buffer(2), function_constant(use_out_source)]], \
      device itype *D [[buffer(3)]], \
      const constant GEMMParams* params [[buffer(4)]], \
      const constant GEMMAddMMParams* addmm_params [[buffer(5), function_constant(use_out_source)]], \
      const constant int* batch_shape [[buffer(6)]], \
      const constant size_t* batch_strides [[buffer(7)]], \
      const constant uint32_t* lhs_indices [[buffer(10), function_constant(do_gather)]], \
      const constant uint32_t* rhs_indices [[buffer(11), function_constant(do_gather)]], \
      const constant uint32_t* C_indices [[buffer(12), function_constant(gather_bias)]], \
      const constant int* operand_shape [[buffer(13), function_constant(do_gather)]], \
      const constant size_t* operand_strides [[buffer(14), function_constant(do_gather)]], \
      const constant packed_int3& operand_batch_ndim [[buffer(15), function_constant(do_gather)]], \
      uint simd_lane_id [[thread_index_in_simdgroup]], \
      uint simd_group_id [[simdgroup_index_in_threadgroup]], \
      uint3 tid [[threadgroup_position_in_grid]], \
      uint3 lid [[thread_position_in_threadgroup]]);

#define instantiate_gemm_transpose_helper(iname, itype, oname, otype, bm, bn, bk, wm, wn) \
    instantiate_gemm(nn, false, false, iname, itype, oname, otype, bm, bn, bk, wm, wn) \
    instantiate_gemm(nt, false, true , iname, itype, oname, otype, bm, bn, bk, wm, wn) \
    instantiate_gemm(tn, true , false, iname, itype, oname, otype, bm, bn, bk, wm, wn) \
    instantiate_gemm(tt, true , true , iname, itype, oname, otype, bm, bn, bk, wm, wn)

// ============================================================
// Tile Configuration: (32, 32, 16, 2, 2) - Original/fallback configuration
// ============================================================
instantiate_gemm_transpose_helper(f32, float, f32, float, 32, 32, 16, 2, 2)
instantiate_gemm_transpose_helper(f16, half, f16, half, 32, 32, 16, 2, 2)
#if defined(__HAVE_BFLOAT__)
instantiate_gemm_transpose_helper(bf16, bfloat, bf16, bfloat, 32, 32, 16, 2, 2)
#endif

// ============================================================
// Tile Configuration: (64, 64, 16, 2, 2) - Default for medium devices
// Reference: MLX steel_gemm_fused.metal
// ============================================================
instantiate_gemm_transpose_helper(f32, float, f32, float, 64, 64, 16, 2, 2)
instantiate_gemm_transpose_helper(f16, half, f16, half, 64, 64, 16, 2, 2)
#if defined(__HAVE_BFLOAT__)
instantiate_gemm_transpose_helper(bf16, bfloat, bf16, bfloat, 64, 64, 16, 2, 2)
#endif

// ============================================================
// Tile Configuration: (64, 64, 16, 1, 2) - For half/bfloat with small K
// Reference: MLX steel_gemm_fused.metal
// ============================================================
instantiate_gemm_transpose_helper(f32, float, f32, float, 64, 64, 16, 1, 2)
instantiate_gemm_transpose_helper(f16, half, f16, half, 64, 64, 16, 1, 2)
#if defined(__HAVE_BFLOAT__)
instantiate_gemm_transpose_helper(bf16, bfloat, bf16, bfloat, 64, 64, 16, 1, 2)
#endif

// ============================================================
// Tile Configuration: (64, 32, 32, 2, 2) - For nt mode with large K
// Reference: MLX steel_gemm_fused.metal
// ============================================================
instantiate_gemm_transpose_helper(f32, float, f32, float, 64, 32, 32, 2, 2)
instantiate_gemm_transpose_helper(f16, half, f16, half, 64, 32, 32, 2, 2)
#if defined(__HAVE_BFLOAT__)
instantiate_gemm_transpose_helper(bf16, bfloat, bf16, bfloat, 64, 32, 32, 2, 2)
#endif

// ============================================================
// Tile Configuration: (32, 64, 16, 1, 2) - For nn mode with large K
// Reference: MLX steel_gemm_fused.metal
// ============================================================
instantiate_gemm_transpose_helper(f32, float, f32, float, 32, 64, 16, 1, 2)
instantiate_gemm_transpose_helper(f16, half, f16, half, 32, 64, 16, 1, 2)
#if defined(__HAVE_BFLOAT__)
instantiate_gemm_transpose_helper(bf16, bfloat, bf16, bfloat, 32, 64, 16, 1, 2)
#endif

// ============================================================
// Fast coalesced M=1 GEMV for [1, K] @ [N, K]^T (nt decode path)
// ============================================================
template <typename T>
[[kernel]] void gemv_nt(
    const device T* A [[buffer(0)]],
    const device T* B [[buffer(1)]],
    device T* D [[buffer(2)]],
    const constant int& N [[buffer(3)]],
    const constant int& K [[buffer(4)]],
    const constant int& ldb [[buffer(5)]],
    uint simd_lane_id [[thread_index_in_simdgroup]],
    uint simd_group_id [[simdgroup_index_in_threadgroup]],
    uint3 tid [[threadgroup_position_in_grid]]) {
  constexpr int SG_PER_TG = 4;
  constexpr int ROWS_PER_SG = 4;
  const int row0 = (int(tid.x) * SG_PER_TG + int(simd_group_id)) * ROWS_PER_SG;
  if (row0 >= N) {
    return;
  }
  const bool has1 = (row0 + 1) < N;
  const bool has2 = (row0 + 2) < N;
  const bool has3 = (row0 + 3) < N;
  const size_t stride = size_t(ldb);
  const device T* w0 = B + size_t(row0) * stride;
  const device T* w1 = has1 ? (w0 + stride) : w0;
  const device T* w2 = has2 ? (w0 + 2 * stride) : w0;
  const device T* w3 = has3 ? (w0 + 3 * stride) : w0;

  float acc0 = 0.0f;
  float acc1 = 0.0f;
  float acc2 = 0.0f;
  float acc3 = 0.0f;

  const int k4 = K & ~3;
  for (int col = int(simd_lane_id) * 4; col < k4; col += 128) {
    float x0 = float(A[col]);
    float x1 = float(A[col + 1]);
    float x2 = float(A[col + 2]);
    float x3 = float(A[col + 3]);

    acc0 = fma(x0, float(w0[col]), acc0);
    acc0 = fma(x1, float(w0[col + 1]), acc0);
    acc0 = fma(x2, float(w0[col + 2]), acc0);
    acc0 = fma(x3, float(w0[col + 3]), acc0);

    if (has1) {
      acc1 = fma(x0, float(w1[col]), acc1);
      acc1 = fma(x1, float(w1[col + 1]), acc1);
      acc1 = fma(x2, float(w1[col + 2]), acc1);
      acc1 = fma(x3, float(w1[col + 3]), acc1);
    }
    if (has2) {
      acc2 = fma(x0, float(w2[col]), acc2);
      acc2 = fma(x1, float(w2[col + 1]), acc2);
      acc2 = fma(x2, float(w2[col + 2]), acc2);
      acc2 = fma(x3, float(w2[col + 3]), acc2);
    }
    if (has3) {
      acc3 = fma(x0, float(w3[col]), acc3);
      acc3 = fma(x1, float(w3[col + 1]), acc3);
      acc3 = fma(x2, float(w3[col + 2]), acc3);
      acc3 = fma(x3, float(w3[col + 3]), acc3);
    }
  }

  for (int c = k4 + int(simd_lane_id); c < K; c += 32) {
    float xv = float(A[c]);
    acc0 = fma(xv, float(w0[c]), acc0);
    if (has1) acc1 = fma(xv, float(w1[c]), acc1);
    if (has2) acc2 = fma(xv, float(w2[c]), acc2);
    if (has3) acc3 = fma(xv, float(w3[c]), acc3);
  }

  acc0 = simd_sum(acc0);
  if (has1) acc1 = simd_sum(acc1);
  if (has2) acc2 = simd_sum(acc2);
  if (has3) acc3 = simd_sum(acc3);

  if (simd_lane_id == 0) {
    D[row0] = static_cast<T>(acc0);
    if (has1) D[row0 + 1] = static_cast<T>(acc1);
    if (has2) D[row0 + 2] = static_cast<T>(acc2);
    if (has3) D[row0 + 3] = static_cast<T>(acc3);
  }
}

template [[host_name("gemv_nt_f32")]] [[kernel]] void gemv_nt<float>(
    const device float* A [[buffer(0)]],
    const device float* B [[buffer(1)]],
    device float* D [[buffer(2)]],
    const constant int& N [[buffer(3)]],
    const constant int& K [[buffer(4)]],
    const constant int& ldb [[buffer(5)]],
    uint simd_lane_id [[thread_index_in_simdgroup]],
    uint simd_group_id [[simdgroup_index_in_threadgroup]],
    uint3 tid [[threadgroup_position_in_grid]]);

template [[host_name("gemv_nt_f16")]] [[kernel]] void gemv_nt<half>(
    const device half* A [[buffer(0)]],
    const device half* B [[buffer(1)]],
    device half* D [[buffer(2)]],
    const constant int& N [[buffer(3)]],
    const constant int& K [[buffer(4)]],
    const constant int& ldb [[buffer(5)]],
    uint simd_lane_id [[thread_index_in_simdgroup]],
    uint simd_group_id [[simdgroup_index_in_threadgroup]],
    uint3 tid [[threadgroup_position_in_grid]]);

#if defined(__HAVE_BFLOAT__)
template [[host_name("gemv_nt_bf16")]] [[kernel]] void gemv_nt<bfloat>(
    const device bfloat* A [[buffer(0)]],
    const device bfloat* B [[buffer(1)]],
    device bfloat* D [[buffer(2)]],
    const constant int& N [[buffer(3)]],
    const constant int& K [[buffer(4)]],
    const constant int& ldb [[buffer(5)]],
    uint simd_lane_id [[thread_index_in_simdgroup]],
    uint simd_group_id [[simdgroup_index_in_threadgroup]],
    uint3 tid [[threadgroup_position_in_grid]]);
#endif

constant float NF4_LUT_TABLE[16] = {
    -1.0f,
    -0.696192800998688f,
    -0.525073051452637f,
    -0.394917488098145f,
    -0.284441381692887f,
    -0.184773430228233f,
    -0.091050036251545f,
    0.0f,
    0.079580299556255f,
    0.160930201411247f,
    0.246112301945686f,
    0.337915241718292f,
    0.440709829330444f,
    0.562617003917694f,
    0.722956836223602f,
    1.0f
};

template <typename T>
METAL_FUNC float nf4_deq_scalar(uint code, float scale) {
  return float(static_cast<T>(NF4_LUT_TABLE[code] * scale));
}

template <typename T>
[[kernel]] void nf4_gemv(
    const device T* A [[buffer(0)]],
    const device uchar* Q [[buffer(1)]],
    const device float* S [[buffer(2)]],
    device T* D [[buffer(3)]],
    const constant int& N [[buffer(4)]],
    const constant int& K [[buffer(5)]],
    uint simd_lane_id [[thread_index_in_simdgroup]],
    uint simd_group_id [[simdgroup_index_in_threadgroup]],
    uint3 tid [[threadgroup_position_in_grid]]) {
  constexpr int SG_PER_TG = 4;
  constexpr int ROWS_PER_SG = 4;
  const int row0 = (int(tid.x) * SG_PER_TG + int(simd_group_id)) * ROWS_PER_SG;
  if (row0 >= N) {
    return;
  }
  const bool has1 = (row0 + 1) < N;
  const bool has2 = (row0 + 2) < N;
  const bool has3 = (row0 + 3) < N;

  const size_t q_row_bytes = size_t(K >> 1);
  const size_t s_row_elems = size_t(K >> 6);

  const device uchar* q0_u8 = Q + size_t(row0) * q_row_bytes;
  const device uchar* q1_u8 = has1 ? (q0_u8 + q_row_bytes) : q0_u8;
  const device uchar* q2_u8 = has2 ? (q0_u8 + 2 * q_row_bytes) : q0_u8;
  const device uchar* q3_u8 = has3 ? (q0_u8 + 3 * q_row_bytes) : q0_u8;

  const device ushort* q0_u16 = reinterpret_cast<const device ushort*>(q0_u8);
  const device ushort* q1_u16 = reinterpret_cast<const device ushort*>(q1_u8);
  const device ushort* q2_u16 = reinterpret_cast<const device ushort*>(q2_u8);
  const device ushort* q3_u16 = reinterpret_cast<const device ushort*>(q3_u8);

  const device float* s0 = S + size_t(row0) * s_row_elems;
  const device float* s1 = has1 ? (s0 + s_row_elems) : s0;
  const device float* s2 = has2 ? (s0 + 2 * s_row_elems) : s0;
  const device float* s3 = has3 ? (s0 + 3 * s_row_elems) : s0;

  float acc0 = 0.0f;
  float acc1 = 0.0f;
  float acc2 = 0.0f;
  float acc3 = 0.0f;

  const int k128 = K & ~127;
  for (int base = 0; base < k128; base += 128) {
    const int col = base + int(simd_lane_id) * 4;
    const float x0 = float(A[col]);
    const float x1 = float(A[col + 1]);
    const float x2 = float(A[col + 2]);
    const float x3 = float(A[col + 3]);

    const int u16_idx = (base >> 2) + int(simd_lane_id);
    const int blk_idx = (base >> 6) + int(simd_lane_id >> 4);

    {
      const ushort p = q0_u16[u16_idx];
      const float sc = s0[blk_idx];
      acc0 = fma(x0, nf4_deq_scalar<T>(p & 0xFu, sc), acc0);
      acc0 = fma(x1, nf4_deq_scalar<T>((p >> 4) & 0xFu, sc), acc0);
      acc0 = fma(x2, nf4_deq_scalar<T>((p >> 8) & 0xFu, sc), acc0);
      acc0 = fma(x3, nf4_deq_scalar<T>((p >> 12) & 0xFu, sc), acc0);
    }
    if (has1) {
      const ushort p = q1_u16[u16_idx];
      const float sc = s1[blk_idx];
      acc1 = fma(x0, nf4_deq_scalar<T>(p & 0xFu, sc), acc1);
      acc1 = fma(x1, nf4_deq_scalar<T>((p >> 4) & 0xFu, sc), acc1);
      acc1 = fma(x2, nf4_deq_scalar<T>((p >> 8) & 0xFu, sc), acc1);
      acc1 = fma(x3, nf4_deq_scalar<T>((p >> 12) & 0xFu, sc), acc1);
    }
    if (has2) {
      const ushort p = q2_u16[u16_idx];
      const float sc = s2[blk_idx];
      acc2 = fma(x0, nf4_deq_scalar<T>(p & 0xFu, sc), acc2);
      acc2 = fma(x1, nf4_deq_scalar<T>((p >> 4) & 0xFu, sc), acc2);
      acc2 = fma(x2, nf4_deq_scalar<T>((p >> 8) & 0xFu, sc), acc2);
      acc2 = fma(x3, nf4_deq_scalar<T>((p >> 12) & 0xFu, sc), acc2);
    }
    if (has3) {
      const ushort p = q3_u16[u16_idx];
      const float sc = s3[blk_idx];
      acc3 = fma(x0, nf4_deq_scalar<T>(p & 0xFu, sc), acc3);
      acc3 = fma(x1, nf4_deq_scalar<T>((p >> 4) & 0xFu, sc), acc3);
      acc3 = fma(x2, nf4_deq_scalar<T>((p >> 8) & 0xFu, sc), acc3);
      acc3 = fma(x3, nf4_deq_scalar<T>((p >> 12) & 0xFu, sc), acc3);
    }
  }

  if (k128 < K) {
    const int col = k128 + int(simd_lane_id) * 2;
    const float x0 = float(A[col]);
    const float x1 = float(A[col + 1]);
    const int byte_idx = (k128 >> 1) + int(simd_lane_id);
    const int blk_idx = k128 >> 6;

    {
      const uchar b = q0_u8[byte_idx];
      const float sc = s0[blk_idx];
      acc0 = fma(x0, nf4_deq_scalar<T>(b & 0xFu, sc), acc0);
      acc0 = fma(x1, nf4_deq_scalar<T>(b >> 4, sc), acc0);
    }
    if (has1) {
      const uchar b = q1_u8[byte_idx];
      const float sc = s1[blk_idx];
      acc1 = fma(x0, nf4_deq_scalar<T>(b & 0xFu, sc), acc1);
      acc1 = fma(x1, nf4_deq_scalar<T>(b >> 4, sc), acc1);
    }
    if (has2) {
      const uchar b = q2_u8[byte_idx];
      const float sc = s2[blk_idx];
      acc2 = fma(x0, nf4_deq_scalar<T>(b & 0xFu, sc), acc2);
      acc2 = fma(x1, nf4_deq_scalar<T>(b >> 4, sc), acc2);
    }
    if (has3) {
      const uchar b = q3_u8[byte_idx];
      const float sc = s3[blk_idx];
      acc3 = fma(x0, nf4_deq_scalar<T>(b & 0xFu, sc), acc3);
      acc3 = fma(x1, nf4_deq_scalar<T>(b >> 4, sc), acc3);
    }
  }

  acc0 = simd_sum(acc0);
  if (has1) acc1 = simd_sum(acc1);
  if (has2) acc2 = simd_sum(acc2);
  if (has3) acc3 = simd_sum(acc3);

  if (simd_lane_id == 0) {
    D[row0] = static_cast<T>(acc0);
    if (has1) D[row0 + 1] = static_cast<T>(acc1);
    if (has2) D[row0 + 2] = static_cast<T>(acc2);
    if (has3) D[row0 + 3] = static_cast<T>(acc3);
  }
}

template [[host_name("nf4_gemv_f32")]] [[kernel]] void nf4_gemv<float>(
    const device float* A [[buffer(0)]],
    const device uchar* Q [[buffer(1)]],
    const device float* S [[buffer(2)]],
    device float* D [[buffer(3)]],
    const constant int& N [[buffer(4)]],
    const constant int& K [[buffer(5)]],
    uint simd_lane_id [[thread_index_in_simdgroup]],
    uint simd_group_id [[simdgroup_index_in_threadgroup]],
    uint3 tid [[threadgroup_position_in_grid]]);

template [[host_name("nf4_gemv_f16")]] [[kernel]] void nf4_gemv<half>(
    const device half* A [[buffer(0)]],
    const device uchar* Q [[buffer(1)]],
    const device float* S [[buffer(2)]],
    device half* D [[buffer(3)]],
    const constant int& N [[buffer(4)]],
    const constant int& K [[buffer(5)]],
    uint simd_lane_id [[thread_index_in_simdgroup]],
    uint simd_group_id [[simdgroup_index_in_threadgroup]],
    uint3 tid [[threadgroup_position_in_grid]]);

#if defined(__HAVE_BFLOAT__)
template [[host_name("nf4_gemv_bf16")]] [[kernel]] void nf4_gemv<bfloat>(
    const device bfloat* A [[buffer(0)]],
    const device uchar* Q [[buffer(1)]],
    const device float* S [[buffer(2)]],
    device bfloat* D [[buffer(3)]],
    const constant int& N [[buffer(4)]],
    const constant int& K [[buffer(5)]],
    uint simd_lane_id [[thread_index_in_simdgroup]],
    uint simd_group_id [[simdgroup_index_in_threadgroup]],
    uint3 tid [[threadgroup_position_in_grid]]);
#endif

template <typename T, int BM>
[[kernel]] void nf4_gemv_batch(
    const device T* A [[buffer(0)]],
    const device uchar* Q [[buffer(1)]],
    const device float* S [[buffer(2)]],
    device T* D [[buffer(3)]],
    const constant int& M [[buffer(4)]],
    const constant int& N [[buffer(5)]],
    const constant int& K [[buffer(6)]],
    uint simd_lane_id [[thread_index_in_simdgroup]],
    uint simd_group_id [[simdgroup_index_in_threadgroup]],
    uint3 tid [[threadgroup_position_in_grid]]) {
  constexpr int SG_PER_TG = 4;
  constexpr int ROWS_PER_SG = 2;
  constexpr int ROWS_PER_TG = SG_PER_TG * ROWS_PER_SG;
  const int row0 = int(tid.x) * ROWS_PER_TG + int(simd_group_id) * ROWS_PER_SG;
  if (row0 >= N || M <= 0 || M > BM) {
    return;
  }
  const int row1 = row0 + 1;
  const bool valid1 = (row1 < N);

  const size_t q_row_bytes = size_t(K >> 1);
  const size_t s_row_elems = size_t(K >> 6);
  const device uchar* q0_u8 = Q + size_t(row0) * q_row_bytes;
  const device ushort* q0_u16 = reinterpret_cast<const device ushort*>(q0_u8);
  const device float* s0_row = S + size_t(row0) * s_row_elems;
  const device uchar* q1_u8 = valid1 ? (q0_u8 + q_row_bytes) : q0_u8;
  const device ushort* q1_u16 = reinterpret_cast<const device ushort*>(q1_u8);
  const device float* s1_row = valid1 ? (s0_row + s_row_elems) : s0_row;

  float acc0[BM];
  float acc1[BM];
#pragma clang loop unroll(full)
  for (int m = 0; m < BM; ++m) {
    acc0[m] = 0.0f;
    acc1[m] = 0.0f;
  }

  const int k128 = K & ~127;
  for (int base = 0; base < k128; base += 128) {
    const int col = base + int(simd_lane_id) * 4;
    const int u16_idx = (base >> 2) + int(simd_lane_id);
    const int blk_idx = (base >> 6) + int(simd_lane_id >> 4);
    const ushort p0 = q0_u16[u16_idx];
    const float sc0 = s0_row[blk_idx];
    const float w00 = nf4_deq_scalar<T>(p0 & 0xFu, sc0);
    const float w01 = nf4_deq_scalar<T>((p0 >> 4) & 0xFu, sc0);
    const float w02 = nf4_deq_scalar<T>((p0 >> 8) & 0xFu, sc0);
    const float w03 = nf4_deq_scalar<T>((p0 >> 12) & 0xFu, sc0);

    if (valid1) {
      const ushort p1 = q1_u16[u16_idx];
      const float sc1 = s1_row[blk_idx];
      const float w10 = nf4_deq_scalar<T>(p1 & 0xFu, sc1);
      const float w11 = nf4_deq_scalar<T>((p1 >> 4) & 0xFu, sc1);
      const float w12 = nf4_deq_scalar<T>((p1 >> 8) & 0xFu, sc1);
      const float w13 = nf4_deq_scalar<T>((p1 >> 12) & 0xFu, sc1);
#pragma clang loop unroll(full)
      for (int m = 0; m < BM; ++m) {
        if (m < M) {
          const device T* am = A + size_t(m) * size_t(K) + size_t(col);
          const float a0 = float(am[0]);
          const float a1 = float(am[1]);
          const float a2 = float(am[2]);
          const float a3 = float(am[3]);
          acc0[m] = fma(a0, w00, acc0[m]);
          acc0[m] = fma(a1, w01, acc0[m]);
          acc0[m] = fma(a2, w02, acc0[m]);
          acc0[m] = fma(a3, w03, acc0[m]);
          acc1[m] = fma(a0, w10, acc1[m]);
          acc1[m] = fma(a1, w11, acc1[m]);
          acc1[m] = fma(a2, w12, acc1[m]);
          acc1[m] = fma(a3, w13, acc1[m]);
        }
      }
    } else {
#pragma clang loop unroll(full)
      for (int m = 0; m < BM; ++m) {
        if (m < M) {
          const device T* am = A + size_t(m) * size_t(K) + size_t(col);
          acc0[m] = fma(float(am[0]), w00, acc0[m]);
          acc0[m] = fma(float(am[1]), w01, acc0[m]);
          acc0[m] = fma(float(am[2]), w02, acc0[m]);
          acc0[m] = fma(float(am[3]), w03, acc0[m]);
        }
      }
    }
  }

  if (k128 < K) {
    const int col = k128 + int(simd_lane_id) * 2;
    const int byte_idx = (k128 >> 1) + int(simd_lane_id);
    const int blk_idx = k128 >> 6;
    const uchar b0 = q0_u8[byte_idx];
    const float sc0 = s0_row[blk_idx];
    const float w00 = nf4_deq_scalar<T>(b0 & 0xFu, sc0);
    const float w01 = nf4_deq_scalar<T>(b0 >> 4, sc0);
#pragma clang loop unroll(full)
    for (int m = 0; m < BM; ++m) {
      if (m < M) {
        const device T* Am = A + size_t(m) * size_t(K) + size_t(col);
        acc0[m] = fma(float(Am[0]), w00, acc0[m]);
        acc0[m] = fma(float(Am[1]), w01, acc0[m]);
      }
    }
    if (valid1) {
      const uchar b1 = q1_u8[byte_idx];
      const float sc1 = s1_row[blk_idx];
      const float w10 = nf4_deq_scalar<T>(b1 & 0xFu, sc1);
      const float w11 = nf4_deq_scalar<T>(b1 >> 4, sc1);
#pragma clang loop unroll(full)
      for (int m = 0; m < BM; ++m) {
        if (m < M) {
          const device T* Am = A + size_t(m) * size_t(K) + size_t(col);
          acc1[m] = fma(float(Am[0]), w10, acc1[m]);
          acc1[m] = fma(float(Am[1]), w11, acc1[m]);
        }
      }
    }
  }

#pragma clang loop unroll(full)
  for (int m = 0; m < BM; ++m) {
    const float total = simd_sum(acc0[m]);
    if (m < M && simd_lane_id == 0) {
      D[size_t(m) * size_t(N) + size_t(row0)] = static_cast<T>(total);
    }
  }
  if (valid1) {
#pragma clang loop unroll(full)
    for (int m = 0; m < BM; ++m) {
      const float total = simd_sum(acc1[m]);
      if (m < M && simd_lane_id == 0) {
        D[size_t(m) * size_t(N) + size_t(row1)] = static_cast<T>(total);
      }
    }
  }
}

#define INSTANTIATE_NF4_GEMV_BATCH(TNAME, TTYPE)                               \
  template [[host_name("nf4_gemv_batch4_" #TNAME)]] [[kernel]] void            \
  nf4_gemv_batch<TTYPE, 4>(                                                    \
      const device TTYPE* A [[buffer(0)]],                                     \
      const device uchar* Q [[buffer(1)]],                                     \
      const device float* S [[buffer(2)]],                                     \
      device TTYPE* D [[buffer(3)]],                                           \
      const constant int& M [[buffer(4)]],                                     \
      const constant int& N [[buffer(5)]],                                     \
      const constant int& K [[buffer(6)]],                                     \
      uint simd_lane_id [[thread_index_in_simdgroup]],                         \
      uint simd_group_id [[simdgroup_index_in_threadgroup]],                   \
      uint3 tid [[threadgroup_position_in_grid]]);                             \
  template [[host_name("nf4_gemv_batch8_" #TNAME)]] [[kernel]] void            \
  nf4_gemv_batch<TTYPE, 8>(                                                    \
      const device TTYPE* A [[buffer(0)]],                                     \
      const device uchar* Q [[buffer(1)]],                                     \
      const device float* S [[buffer(2)]],                                     \
      device TTYPE* D [[buffer(3)]],                                           \
      const constant int& M [[buffer(4)]],                                     \
      const constant int& N [[buffer(5)]],                                     \
      const constant int& K [[buffer(6)]],                                     \
      uint simd_lane_id [[thread_index_in_simdgroup]],                         \
      uint simd_group_id [[simdgroup_index_in_threadgroup]],                   \
      uint3 tid [[threadgroup_position_in_grid]]);                             \
  template [[host_name("nf4_gemv_batch16_" #TNAME)]] [[kernel]] void           \
  nf4_gemv_batch<TTYPE, 16>(                                                   \
      const device TTYPE* A [[buffer(0)]],                                     \
      const device uchar* Q [[buffer(1)]],                                     \
      const device float* S [[buffer(2)]],                                     \
      device TTYPE* D [[buffer(3)]],                                           \
      const constant int& M [[buffer(4)]],                                     \
      const constant int& N [[buffer(5)]],                                     \
      const constant int& K [[buffer(6)]],                                     \
      uint simd_lane_id [[thread_index_in_simdgroup]],                         \
      uint simd_group_id [[simdgroup_index_in_threadgroup]],                   \
      uint3 tid [[threadgroup_position_in_grid]]);

INSTANTIATE_NF4_GEMV_BATCH(f32, float)
INSTANTIATE_NF4_GEMV_BATCH(f16, half)
#if defined(__HAVE_BFLOAT__)
INSTANTIATE_NF4_GEMV_BATCH(bf16, bfloat)
#endif
#undef INSTANTIATE_NF4_GEMV_BATCH

template <typename T>
[[kernel]] void nf4_dequant(
    const device uchar* Q [[buffer(0)]],
    const device float* S [[buffer(1)]],
    device T* Out [[buffer(2)]],
    const constant uint& num_bytes [[buffer(3)]],
    uint gid [[thread_position_in_grid]]) {
  if (gid >= num_bytes) {
    return;
  }
  const uchar b = Q[gid];
  const float scale = S[gid >> 5];
  const size_t out_idx = size_t(gid) << 1;
  Out[out_idx] = static_cast<T>(NF4_LUT_TABLE[b & 0xFu] * scale);
  Out[out_idx + 1] = static_cast<T>(NF4_LUT_TABLE[b >> 4] * scale);
}

template [[host_name("nf4_dequant_f32")]] [[kernel]] void nf4_dequant<float>(
    const device uchar* Q [[buffer(0)]],
    const device float* S [[buffer(1)]],
    device float* Out [[buffer(2)]],
    const constant uint& num_bytes [[buffer(3)]],
    uint gid [[thread_position_in_grid]]);

template [[host_name("nf4_dequant_f16")]] [[kernel]] void nf4_dequant<half>(
    const device uchar* Q [[buffer(0)]],
    const device float* S [[buffer(1)]],
    device half* Out [[buffer(2)]],
    const constant uint& num_bytes [[buffer(3)]],
    uint gid [[thread_position_in_grid]]);

#if defined(__HAVE_BFLOAT__)
template [[host_name("nf4_dequant_bf16")]] [[kernel]] void nf4_dequant<bfloat>(
    const device uchar* Q [[buffer(0)]],
    const device float* S [[buffer(1)]],
    device bfloat* Out [[buffer(2)]],
    const constant uint& num_bytes [[buffer(3)]],
    uint gid [[thread_position_in_grid]]);
#endif
