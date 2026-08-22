# CBC / normalized-extraction experiments: reproducibility package

这个目录包含论文第二轮实验的最小可复现脚本。

## 文件

- `reproduce_cyclo_estimator.py`
  - 复现 Cyclo `estimates.ipynb` 所用的欧氏 SIS 估计路径；
  - 使用与仓库固定的 lattice-estimator 提交相同的根 Hermite 因子和 MATZOV 成本公式；
  - 验证 notebook 缓存基线约 `127.1 bits`；
  - 扫描 `L=1,...,8` 下 full-ternary 和 fixed-weight-32 的严格全局半径；
  - 输出 128-bit 门槛下的最小 Ajtai rank。

- `benchmark_integer_ipa.py`
  - 使用论文中旧/新 `log2 q` 公式；
  - 实测 Python 任意精度整数的 base-q Horner 编码；
  - 使用 2048/3072 位模幂作为未知阶群标量乘的微基准代理；
  - old/new 对比复用同一 coefficient vector、同一模数和同一确定性随机流，避免输入差异污染加速比；
  - 额外测量无预计算时 `Enc_q(x)` 量级的大指数模幂。
  - 这不是 Fu 协议的端到端 benchmark。

- `run_all.py`
  - 顺序运行上述两个实验。

## 运行

```bash
python3 -m pip install pandas matplotlib
python3 run_all.py
```

结果会写入：

```text
results/
```

## 重要口径

### Cyclo

当前严谨全局 SIS 半径使用：

```text
B_global = 8 * beta_hat * gamma
beta_hat = 1024 + 2 * 64 * L * gamma
```

full ternary 使用 `gamma=128`，fixed-weight-32 使用 `gamma=32`。

不要把条件性的 local-reference 半径 `29.4046` 作为 estimator 的全局输入。

### Integer IPA

旧参数：

```text
log2 q_old
= [2r + 16 + 8 log2 r] lambda
  + 64 r^2 + log2 B + 9.
```

新 CBC 参数：

```text
log2 q_new
= [2r + 8 + 3 log2 r + log2(r-1)] lambda
  + 32 r^2 - 16r + 11 + log2 B.
```

默认微基准参数：

```text
lambda = 128
log2 B = 64
r = 2,...,8
```

模幂只用于衡量“更短标量”的真实大整数/群运算影响，不应表述成完整 prover/verifier 加速。

## 预期 sanity check

Cyclo 原 notebook 基线应接近：

```text
security ≈ 127.086 bits
BKZ beta = 337
d = 3591
delta ≈ 1.00447856
```

在当前机器上，整数 IPA 微基准的绝对时间会随 CPU 和 Python 版本变化，因此论文更适合报告：
- old/new 参数位长；
- 加速倍数；
- 多次重复后的中位数；
而非跨机器比较绝对秒数。
