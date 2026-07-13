# UPDRS Proxy Score — Methodology & Clinical Validation Protocol

**PhysicSense Research Document · Draft v0.3**
Module: `core/neuromotor` · Scoring: `scoring.rs`

---

## 1. Background

The Unified Parkinson's Disease Rating Scale (UPDRS) is the gold-standard
clinical tool for quantifying Parkinson's Disease (PD) motor severity.
Part III (motor examination) requires a trained neurologist and takes
20–40 minutes per session, making longitudinal monitoring impractical.

PhysicSense computes a **proxy UPDRS score** from ambient RF and acoustic
signals — no wearable, no specialist, no clinic visit. The proxy maps to
two UPDRS Part III sub-items most sensitive to tremor and gait:

| UPDRS Item | Description                        | Max Score | PhysicSense Sub-score |
|------------|------------------------------------|-----------|----------------------|
| 20         | Tremor at rest (hands)             | 4         | `tremor_subscore`    |
| 29–30      | Gait & postural instability        | 4         | `gait_subscore`      |
| **Total**  |                                    | **8**     | `composite`          |

A composite ≥ 3.0 triggers a **clinical flag** (`flag_for_review = true`),
indicating referral for formal neurological examination is advisable.

> **Disclaimer**: The proxy score is a screening tool only.
> It is not a diagnostic instrument and must not replace clinical assessment.

---

## 2. Tremor Sub-score (`tremor_subscore`, 0–4)

### 2.1 Signal acquisition

The system samples 1 second of micro-Doppler velocity data at 1 kHz
(from the WiFi bistatic pipeline or acoustic FMCW, whichever has higher
SNR) and applies a three-band biquad IIR filter bank:

| Band            | Centre | BW   | Clinical relevance                    |
|-----------------|--------|------|---------------------------------------|
| Parkinsonian    | 4.5 Hz | 3 Hz | Resting tremor, 3–6 Hz (UPDRS item 20)|
| Essential       | 8 Hz   | 8 Hz | Action/postural tremor, 4–12 Hz       |
| Physiological   | 10 Hz  | 4 Hz | Normal physiological noise, 8–12 Hz   |

### 2.2 Classification

Normalised band power: `p_i = power_i / Σ power_j`

- **Parkinsonian** if `p_park > 0.40`
- **Essential**    if `p_ess  > 0.40`
- **Physiological** if `p_phys > 0.40` (and neither of above)
- **Indeterminate** if no band exceeds 0.40

### 2.3 Score mapping

| Tremor class   | tremor_subscore | Rationale                          |
|----------------|-----------------|------------------------------------|
| None           | 0.0             | No tremor detected                 |
| Physiological  | 0.5             | Normal; not clinically significant |
| Indeterminate  | 1.0             | Low confidence — borderline signal |
| Essential      | 1.5             | Action tremor; non-PD aetiology    |
| Parkinsonian   | 2.0–4.0*        | Scaled by dominant-band power      |

*Parkinsonian sub-score: `2.0 + 2.0 × (p_park − 0.40) / 0.60`
(linear scaling from threshold to full band dominance)

---

## 3. Gait Sub-score (`gait_subscore`, 0–4)

### 3.1 Metrics extracted

| Metric              | Symbol | Normal range      | Source           |
|---------------------|--------|-------------------|------------------|
| Cadence             | C      | 100–130 spm       | Step detector    |
| Asymmetry Index     | AI     | < 0.05            | Stride-pair ratio|
| Stride CV           | CV     | < 3 %             | Inter-stride σ/μ |
| FOG Risk            | FOG    | Low / Med / High  | Cadence + CV     |

### 3.2 Score mapping

Each metric contributes up to 1.0 point:

```
score_cadence  = 0  if 90 ≤ C ≤ 140 else clamp(|C − 115| / 30, 0, 1)
score_asym     = clamp(AI / 0.15, 0, 1)
score_cv       = clamp(CV / 8.0,  0, 1)
score_fog      = { Low: 0, Medium: 0.5, High: 1.0 }

gait_subscore  = score_cadence + score_asym + score_cv + score_fog
```

Capped at 4.0.

---

## 4. Clinical Validation Protocol

### 4.1 Target cohort

| Group      | n  | Criteria                                  |
|------------|----|-------------------------------------------|
| PD Stage I | 20 | Hoehn & Yahr I; UPDRS III 1–20           |
| PD Stage II| 20 | Hoehn & Yahr II; UPDRS III 20–40         |
| ET         | 15 | Essential tremor, no PD diagnosis         |
| Control    | 25 | No neurological history, age-matched      |

### 4.2 Protocol

1. Participant sits at rest, 2 m from PhysicSense node, 5-min session
2. Clinician scores UPDRS Part III (blinded to PhysicSense output)
3. PhysicSense proxy score computed post-hoc
4. Pearson r and Bland-Altman analysis vs. ground-truth UPDRS items 20, 29–30

### 4.3 Target performance

| Metric                  | Target     |
|-------------------------|------------|
| Pearson r (vs UPDRS 20) | ≥ 0.75     |
| Sensitivity (flag ≥3.0) | ≥ 0.85     |
| Specificity             | ≥ 0.80     |
| Mean Absolute Error     | ≤ 0.8 pts  |

---

## 5. Comparison with Prior Art

| System            | Modality        | Tremor Hz | UPDRS | No hardware | Reference          |
|-------------------|----------------|-----------|-------|-------------|--------------------|
| Smartwatch + ML   | IMU wrist       | 3–12      | Proxy | No          | Arora 2015         |
| Spiral drawing    | Touchscreen     | —         | No    | No          | Memedi 2015        |
| RF-Pose (MIT)     | Active 60 GHz   | No        | No    | No          | Zhao 2018          |
| WiGait (MIT)      | Active 5.46 GHz | No        | No    | No          | Zhao 2017          |
| **PhysicSense**   | Passive WiFi + acoustic | 3–12 | **Yes** | **Yes** | This work      |

PhysicSense is the first system to combine passive RF sensing with
neuromotor screening without any on-body or active-transmission hardware.

---

## 6. Limitations

- SNR degrades beyond 4 m from subject; effective range 1–3 m
- Multi-person scenes require source separation (future work)
- Proxy score validated only against simulated data at this stage;
  clinical cohort study is planned (see §4)
- Clothing and body mass affect micro-Doppler return amplitude

---

## 7. References

1. Goetz CG et al. "Movement Disorder Society-Sponsored Revision of the
   Unified Parkinson's Disease Rating Scale." *Mov Disord* 2008.
2. Zhao R et al. "Through-Wall Human Pose Estimation Using Radio Signals."
   *CVPR* 2018.
3. Adib F et al. "Smart Homes that Monitor Breathing and Heart Rate."
   *CHI* 2015.
4. Arora S et al. "Detecting and monitoring the symptoms of Parkinson's
   disease using smartphones." *J Neurosci Methods* 2015.
