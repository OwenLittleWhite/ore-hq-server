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

# 创建用于存储 Metabase 数据的目录
mkdir -p ~/metabase-data

# 运行 Metabase 容器
docker run -d \
  --name metabase \
  -p 5000:3000 \
  -v ~/metabase-data:/metabase-data \
  -e "MB_DB_FILE=/metabase-data/metabase.db" \
  metabase/metabase

sh -c "$(curl -sSfL https://release.solana.com/v1.16.25/install)"

写一个sh脚本，能够批量管理启动的服务进程
启动单个服务的命令行为:
mkdir hq-server1 && cd hq-server1 && /home/ore-hq-server --port 8000 --miner-ids 1,2 --priority-fee 1000
还要在对应的目录下创建.env文件，内容如下:
WALLET_PATH = "/home/owen/.config/solana/id2.json"
RPC_URL = "https://bitz-000.eclipserpc.xyz/"
RPC_WS_URL = "wss://bitz-000.eclipserpc.xyz/"
PASSWORD = "123456"
DATABASE_URL = "mysql://root:123456@127.0.0.1:3306/money"



其中目录命名能够以1序列进行增长，端口以8000序列增长，WALLET_PATH以id1.json增长，其他保持不变

提供可交互式的体验，能够批量启动服务、停止单个服务、重启单个服务、全部停止、全部重启、列出服务状态、移除服务、批量移除服务

insert into miners (pubkey, enabled) values ("uK7CPbypEjkJeAg2AJynVG67ZFxu2xJ3YSQYUsjdhhA", 1),( "7mEkNxSKtyHMZf6kA9siQDm5jSePr5LxNGgbtAwYz2ue", 1) ;

insert into rewards (miner_id, pool_id, balance) values (1,1,0), (2,1,0), (1,2,0),(2,2,0),(1,3,0), (2,3,0);
