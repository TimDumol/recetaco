#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
export ANSIBLE_LOCAL_TEMP="${ANSIBLE_LOCAL_TEMP:-/tmp/recetaco-ansible}"
IMAGE="recetaco:release-$(date -u +%Y%m%d%H%M%S)"
# Keep the image and all build context free of provider credentials.
cargo test --locked
python3 scripts/configure-pocket-id.py
docker build --platform linux/amd64 -t "$IMAGE" .
docker run --rm --entrypoint /bin/true "$IMAGE"
docker save "$IMAGE" | gzip -1 | ssh -i /home/timdumol/.ssh/aprendiendo-deploy \
  -o BatchMode=yes timdumol@77.42.68.214 'gzip -d | sudo -n docker load'
ansible-playbook -i deploy/ansible/inventory.yml deploy/ansible/site.yml -e "recetaco_image=$IMAGE"
python3 scripts/check-deployment.py
