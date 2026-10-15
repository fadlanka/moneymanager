#!/usr/bin/env bash
# Nightly database + receipt photo backup. Example crontab (03:15 every day):
#   15 3 * * * cd /opt/moneymanager && ./deploy/backup.sh >> backups/backup.log 2>&1
set -euo pipefail

cd "$(dirname "$0")/.."
mkdir -p backups
stamp=$(date +%Y%m%d-%H%M)

docker compose exec -T db sh -c 'pg_dump -U "$POSTGRES_USER" -d moneymanager -Fc' > "backups/db-$stamp.dump"
docker compose run --rm -T --no-deps --user root -v "$PWD/backups:/backup" --entrypoint sh api \
  -c "tar czf /backup/uploads-$stamp.tgz -C /data uploads"

# Keep two weeks.
find backups -name 'db-*.dump' -mtime +14 -delete
find backups -name 'uploads-*.tgz' -mtime +14 -delete
echo "backup $stamp done"
