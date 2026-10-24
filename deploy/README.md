# Production

Run `scripts/release.sh` from a machine with Docker, Rust, Ansible, Python 3, and SSH
access to zeus (`77.42.68.214`, account `timdumol`, deployment key
`~/.ssh/aprendiendo-deploy`). The release configures Pocket ID, builds the
linux/amd64 image, streams it over SSH, and deploys it with Ansible.

The app lives at https://recetaco.timdumol.com. UI, API, and MCP share that origin;
`api.recetaco.timdumol.com` is unnecessary. The only edge proxy is the existing
`app-caddy-1`: Recetaco joins `caddy-edge`, publishes no host ports, and installs
only `/opt/caddy/sites-enabled/recetaco.caddy`. Ansible validates the complete
Caddy configuration and reloads it without restarting the shared container.

Pocket ID: https://auth.aries.timdumol.com. The ignored root file
`.pocket-id-api-key` is used locally for provisioning only; it is never copied
into the image or onto the application server. Browser client `recetaco-web`
uses `https://recetaco.timdumol.com/auth/callback` and logout callback
`https://recetaco.timdumol.com`. Android client `recetaco-android` uses
`dev.recetaco.app:/oauth2redirect`. Both are public PKCE clients restricted to
the `recetaco-household` group (Tim and Jopi), with delegated access to the
`https://recetaco.timdumol.com` API. The server also permits only those two subjects.
The provisioning script owns that group's membership and these two client IDs.

Data persists in `/opt/recetaco/data/recetaco.db`, including photos. Use SQLite's
backup API for a live backup, or stop the Recetaco service before copying the
database. Do not remove the data directory when updating or rolling back.

To roll back, supply a previously loaded image:

```sh
ansible-playbook -i deploy/ansible/inventory.yml deploy/ansible/site.yml \
  -e recetaco_image=recetaco:release-PREVIOUS
```

The Ansible playbook assumes Docker Compose and shared Caddy are already
provisioned by Aprendiendo. It never manages other services or their data.

Run `python3 scripts/check-deployment.py` to repeat the read-only public checks.
Passkey sign-in still requires using your browser or Android device.
