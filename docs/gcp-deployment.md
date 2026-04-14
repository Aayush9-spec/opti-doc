# GCP Deployment Guide — OptiDock AI

Complete setup guide for deploying OptiDock AI to Google Cloud Platform.
Since `gcloud` CLI is not available, all setup is done through the **GCP Web Console** and **GitHub Actions**.

---

## Prerequisites

- Google Cloud account with billing enabled
- GitHub repository: `Aayush9-spec/opti-doc`
- GCP Project ID: `nth-bounty-477010-h8`

---

## Step 1: Enable Required APIs

Go to [GCP Console → APIs & Services](https://console.cloud.google.com/apis/library?project=nth-bounty-477010-h8)

Enable these APIs (click each link):

1. [Cloud Run Admin API](https://console.cloud.google.com/apis/library/run.googleapis.com?project=nth-bounty-477010-h8)
2. [Artifact Registry API](https://console.cloud.google.com/apis/library/artifactregistry.googleapis.com?project=nth-bounty-477010-h8)
3. [Cloud Build API](https://console.cloud.google.com/apis/library/cloudbuild.googleapis.com?project=nth-bounty-477010-h8)
4. [IAM Service Account Credentials API](https://console.cloud.google.com/apis/library/iamcredentials.googleapis.com?project=nth-bounty-477010-h8)

> Click the **"Enable"** button on each page. Wait for each to finish before moving on.

---

## Step 2: Create Artifact Registry Repository

1. Go to [Artifact Registry](https://console.cloud.google.com/artifacts?project=nth-bounty-477010-h8)
2. Click **"+ CREATE REPOSITORY"**
3. Fill in:
   - **Name:** `optidock`
   - **Format:** `Docker`
   - **Mode:** `Standard`
   - **Location type:** `Region`
   - **Region:** `us-central1`
4. Click **"CREATE"**

---

## Step 3: Create a Service Account for GitHub Actions

1. Go to [IAM → Service Accounts](https://console.cloud.google.com/iam-admin/serviceaccounts?project=nth-bounty-477010-h8)
2. Click **"+ CREATE SERVICE ACCOUNT"**
3. Fill in:
   - **Name:** `github-deploy`
   - **ID:** `github-deploy`
   - **Description:** `GitHub Actions CI/CD deployment`
4. Click **"CREATE AND CONTINUE"**
5. Add these roles (click "Add Another Role" for each):
   - `Cloud Run Admin`
   - `Artifact Registry Writer`
   - `Service Account User`
   - `Storage Admin`
6. Click **"CONTINUE"** → **"DONE"**

---

## Step 4: Generate a Service Account Key

1. In the Service Accounts list, click on `github-deploy@nth-bounty-477010-h8.iam.gserviceaccount.com`
2. Go to the **"KEYS"** tab
3. Click **"ADD KEY"** → **"Create new key"**
4. Select **JSON** format
5. Click **"CREATE"**
6. A JSON file will download — **save this file, you'll need it next**

---

## Step 5: Add the Key to GitHub Secrets

1. Go to your GitHub repo: [Settings → Secrets → Actions](https://github.com/Aayush9-spec/opti-doc/settings/secrets/actions)
2. Click **"New repository secret"**
3. **Name:** `GCP_SA_KEY`
4. **Value:** Open the downloaded JSON file in a text editor, select ALL the content, and paste it here
5. Click **"Add secret"**

---

## Step 6: Push to Main and Deploy

Once all the above is done, just push your code to the `main` branch:

```bash
git add .
git commit -m "feat: add GCP Cloud Run deployment"
git push origin main
```

GitHub Actions will automatically:
1. Build both Docker images (Rust CLI + Next.js landing)
2. Push them to Google Artifact Registry
3. Deploy both to Cloud Run
4. Print the live URLs in the workflow summary

---

## Step 7: Find Your Live URLs

After the GitHub Actions workflow completes:

1. Go to [Cloud Run Console](https://console.cloud.google.com/run?project=nth-bounty-477010-h8)
2. You'll see two services:
   - **optidock-cli** — The Rust API (click to see the URL)
   - **optidock-landing** — The landing page (click to see the URL)

Each service gets a URL like:
- `https://optidock-cli-XXXXX-uc.a.run.app`
- `https://optidock-landing-XXXXX-uc.a.run.app`

---

## Viewing in Google Cloud Console

### Cloud Run Dashboard
[console.cloud.google.com/run](https://console.cloud.google.com/run?project=nth-bounty-477010-h8)
- View deployed services, revisions, traffic splitting
- See request logs and error rates
- Configure custom domains

### Artifact Registry
[console.cloud.google.com/artifacts](https://console.cloud.google.com/artifacts?project=nth-bounty-477010-h8)
- View stored Docker images
- See image tags and versions

### Cloud Build History
[console.cloud.google.com/cloud-build](https://console.cloud.google.com/cloud-build/builds?project=nth-bounty-477010-h8)
- View build logs and history

### Logs Explorer
[console.cloud.google.com/logs](https://console.cloud.google.com/logs?project=nth-bounty-477010-h8)
- View real-time application logs
- Filter by service name

---

## API Endpoints (After Deployment)

The CLI service exposes these endpoints:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/` | GET | Service status and version |
| `/health` | GET | Health check (Cloud Run uses this) |
| `/analyze` | GET | Run Dockerfile analysis |
| `/analyze?path=./sample` | GET | Analyze a specific path |
| `/pipeline` | GET | Run pipeline moderation |
| `/providers` | GET | List AI provider configuration |

---

## Troubleshooting

### Build fails in GitHub Actions
- Check that `GCP_SA_KEY` secret is set correctly (full JSON content)
- Verify all 4 APIs are enabled in Step 1

### Cloud Run returns 404 or 500
- Check [Cloud Run Logs](https://console.cloud.google.com/run/detail/us-central1/optidock-cli/logs?project=nth-bounty-477010-h8) for error details
- The container needs about 10-20 seconds for cold start (Rust binary startup)

### Permission denied errors
- Make sure the service account has all 4 roles from Step 3
- Re-download the JSON key if needed

---

## Cost

Cloud Run free tier includes:
- 2 million requests/month
- 360,000 GB-seconds of memory
- 180,000 vCPU-seconds

Your project will likely stay within the free tier.
