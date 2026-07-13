# Clinical Validation Protocol — PhysicSense Neuromotor Module

**Version**: 0.2 · **Status**: Draft — pending ethics approval
**Contact**: abhisheksjayanth@gmail.com

---

## 1. Study Objectives

1. Quantify the agreement between PhysicSense proxy UPDRS scores and
   clinician-administered UPDRS Part III scores (items 20, 29–30)
2. Determine sensitivity/specificity for flagging scores ≥ 3.0
3. Characterise performance across Hoehn & Yahr stages I–II and an
   age-matched healthy control group

---

## 2. Inclusion / Exclusion Criteria

**Inclusion**
- Age 40–80
- Confirmed PD diagnosis (MDS criteria) OR Essential Tremor diagnosis
  OR healthy control (no neurological history)
- Able to sit unsupported for 5 minutes

**Exclusion**
- Active infection or fever
- Cardiac pacemaker (ultrasound transducer contraindication)
- Inability to provide informed consent

---

## 3. Session Procedure

```
T=0 min   Participant enters room, seated 2 m from sensor node
T=1 min   PhysicSense calibration (30 s background capture)
T=2 min   5-minute passive sensing session begins (subject at rest)
T=7 min   Subject performs 10-metre walk (gait capture)
T=9 min   Session ends; PhysicSense score computed
T=10 min  Clinician performs UPDRS Part III (blinded to proxy score)
T=30 min  De-brief; scores compared by study coordinator
```

---

## 4. Data Collection

| Field               | Source            | Format       |
|---------------------|-------------------|--------------|
| Subject ID          | Study coordinator | Anonymised   |
| Age, sex, H&Y stage | Clinical record   | Structured   |
| Clinician UPDRS     | Neurologist       | Items 20,29,30|
| PhysicSense proxy   | Automated         | JSON         |
| Raw IQ / acoustic   | Sensor node       | Binary (opt) |

---

## 5. Statistical Analysis Plan

- **Primary**: Pearson r between proxy composite and UPDRS (items 20+29+30)
- **Secondary**: Bland-Altman plot; ROC curve for flag threshold
- **Subgroup**: PD vs ET vs Control; Stage I vs Stage II
- **Sample size**: 80 participants; power 0.80, α=0.05, expected r=0.75

---

## 6. Ethics & Privacy

- Study will be submitted to institutional ethics board before recruitment
- All data stored encrypted at rest (AES-256); anonymised before analysis
- Raw WiFi IQ frames deleted after feature extraction unless participant
  consents to data retention
- No data shared with third parties

---

## 7. Timeline

| Milestone                        | Target date     |
|----------------------------------|-----------------|
| Ethics submission                | Month 3         |
| Ethics approval                  | Month 5         |
| Pilot (n=10)                     | Month 6         |
| Full cohort recruitment complete | Month 10        |
| Analysis complete                | Month 12        |
| Manuscript submission            | Month 14        |
