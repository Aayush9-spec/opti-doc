#!/bin/bash
set -e

PROJECT_ID="nth-bounty-477010-h8"
REGION="us-central1"
CLI_SERVICE="optidock-cli"
LANDING_SERVICE="optidock-landing"
AR_REPO="optidock"

echo "=============================================="
echo "🚀 OptiDock AI - Enterprise GCP Deployer 🚀"
echo "=============================================="

echo "[1/6] Enabling required Enterprise APIs..."
gcloud services enable run.googleapis.com \
  cloudbuild.googleapis.com \
  artifactregistry.googleapis.com \
  secretmanager.googleapis.com \
  containerscanning.googleapis.com \
  --project=$PROJECT_ID

gcloud config set project $PROJECT_ID

echo "[2/6] Setting up Secret Manager..."
# Check or create Supabase URL secret
if ! gcloud secrets describe SUPABASE_URL --project=$PROJECT_ID >/dev/null 2>&1; then
  echo "Creating SUPABASE_URL secret..."
  read -p "Enter your NEXT_PUBLIC_SUPABASE_URL: " secret_val
  echo -n "$secret_val" | gcloud secrets create SUPABASE_URL --data-file=- --project=$PROJECT_ID
fi

# Check or create Gemini Key secret
if ! gcloud secrets describe GEMINI_API_KEY --project=$PROJECT_ID >/dev/null 2>&1; then
  echo "Creating GEMINI_API_KEY secret..."
  read -p "Enter your GEMINI_API_KEY: " secret_val
  echo -n "$secret_val" | gcloud secrets create GEMINI_API_KEY --data-file=- --project=$PROJECT_ID
fi

echo "[3/6] Granting Compute Engine Service Account access to Secrets..."
PROJECT_NUM=$(gcloud projects describe $PROJECT_ID --format="value(projectNumber)")
COMPUTE_SA="${PROJECT_NUM}-compute@developer.gserviceaccount.com"

gcloud secrets add-iam-policy-binding SUPABASE_URL \
    --member="serviceAccount:${COMPUTE_SA}" \
    --role="roles/secretmanager.secretAccessor" \
    --project=$PROJECT_ID >/dev/null

gcloud secrets add-iam-policy-binding GEMINI_API_KEY \
    --member="serviceAccount:${COMPUTE_SA}" \
    --role="roles/secretmanager.secretAccessor" \
    --project=$PROJECT_ID >/dev/null

echo "[4/6] Creating Artifact Registry & Enabling Scanning..."
gcloud artifacts repositories create $AR_REPO \
  --repository-format=docker \
  --location=$REGION \
  --description="OptiDock Registry" || true

echo "[5/6] Deploying OptiDock CLI API to Cloud Run..."
gcloud run deploy $CLI_SERVICE \
  --source . \
  --region $REGION \
  --allow-unauthenticated \
  --project $PROJECT_ID \
  --set-env-vars=RUST_LOG=info,OPTIDOCK_PROVIDER=gemini \
  --set-secrets="GEMINI_API_KEY=GEMINI_API_KEY:latest" \
  --memory=512Mi \
  --quiet

echo "[6/6] Deploying OptiDock Landing Page to Cloud Run..."
gcloud run deploy $LANDING_SERVICE \
  --source ./landing \
  --region $REGION \
  --allow-unauthenticated \
  --project $PROJECT_ID \
  --set-env-vars=NODE_ENV=production \
  --set-secrets="NEXT_PUBLIC_SUPABASE_URL=SUPABASE_URL:latest" \
  --memory=256Mi \
  --quiet

echo "=============================================="
echo "✅ Enterprise Deployment Successful!"
echo "=============================================="
