#!/usr/bin/env bash
#
# Runs on the EC2 host (invoked by deploy.yml via SSM Run Command).
# Pulls the image tag requested by the CI job, then rolls the compose
# stack. Uses the instance profile's ECR pull permissions — no
# long-lived docker credentials anywhere on disk.
#
# Installed by provisioning at /opt/sf-clone/deploy.sh.
# Invoked as:  /opt/sf-clone/deploy.sh <git-sha>

set -euo pipefail

IMAGE_TAG="${1:-latest}"

# /opt/sf-clone/.env sets IMAGE_REGISTRY, IMAGE_REPO, AWS_REGION,
# POSTGRES_PASSWORD, JWT_SECRET, etc. Compose reads it automatically,
# but the ECR-login line below needs them up-front.
if [[ -f /opt/sf-clone/.env ]]; then
    set -a
    # shellcheck disable=SC1091
    source /opt/sf-clone/.env
    set +a
fi

: "${AWS_REGION:?AWS_REGION not set in /opt/sf-clone/.env}"
: "${IMAGE_REGISTRY:?IMAGE_REGISTRY not set in /opt/sf-clone/.env}"
: "${IMAGE_REPO:?IMAGE_REPO not set in /opt/sf-clone/.env}"

cd /opt/sf-clone

echo "==> logging into ECR (${IMAGE_REGISTRY})"
aws ecr get-login-password --region "$AWS_REGION" \
  | docker login --username AWS --password-stdin "$IMAGE_REGISTRY"

export IMAGE_TAG
echo "==> pulling ${IMAGE_REGISTRY}/${IMAGE_REPO}:${IMAGE_TAG}"
docker compose -f docker-compose.prod.yml pull

echo "==> rolling services"
docker compose -f docker-compose.prod.yml up -d --remove-orphans

echo "==> pruning dangling images"
docker image prune -f

echo "==> deploy done: ${IMAGE_TAG}"
