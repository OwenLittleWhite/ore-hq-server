## 部署文档

cargo install cargo-binstall

cargo binstall diesel_cli
sudo apt-get install libmysqlclient-dev

cargo install diesel_cli@1.4.1 --no-default-features --features mysql  
./ore-hq-server --port 3500 --miner-ids 1,2 --priority-fee 1000

docker pull mysql:5.7  

docker run -d \
  --name mysql5.7 \
  -p 3306:3306 \
  -e MYSQL_ROOT_PASSWORD=123456 \
  -e MYSQL_ROOT_HOST=% \
  -v /path/to/mysql/data:/var/lib/mysql \
  mysql:5.7 \
  --character-set-server=utf8mb4 \
  --collation-server=utf8mb4_unicode_ci  

