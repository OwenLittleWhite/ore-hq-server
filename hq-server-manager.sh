#!/bin/bash

# 基础配置
BASE_DIR=$(pwd)
CONFIG_FILE="${BASE_DIR}/hq-server-manager.cfg"
SERVER_BINARY="/home/owen/ore-hq-server/ore-hq-server"
BASE_PORT=9000
WALLET_BASE="/home/owen/.config/solana/id"
WALLET_EXT=".json"
ENV_TEMPLATE="WALLET_PATH = \"%s\"
RPC_URL = \"https://bitz-000.eclipserpc.xyz/\"
RPC_WS_URL = \"wss://bitz-000.eclipserpc.xyz/\"
PASSWORD = \"123456\"
DATABASE_URL = \"mysql://root:123456@127.0.0.1:3306/money\""

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 初始化配置文件
init_config() {
    if [ ! -f "$CONFIG_FILE" ]; then
        touch "$CONFIG_FILE"
        echo "# HQ Server Manager Configuration" > "$CONFIG_FILE"
        echo "# Format: SERVER_NUM|PORT|WALLET_PATH|DIR_NAME" >> "$CONFIG_FILE"
    fi
}

# 添加服务到配置
add_server_to_config() {
    local server_num=$1
    local port=$2
    local wallet_path=$3
    local dir_name=$4
    
    # 检查是否已存在
    if grep -q "^${server_num}|" "$CONFIG_FILE"; then
        sed -i "/^${server_num}|/d" "$CONFIG_FILE"
    fi
    
    echo "${server_num}|${port}|${wallet_path}|${dir_name}" >> "$CONFIG_FILE"
}

# 从配置中移除服务
remove_server_from_config() {
    local server_num=$1
    sed -i "/^${server_num}|/d" "$CONFIG_FILE"
}

# 从配置中获取所有服务
get_all_servers() {
    awk -F'|' '{print $1}' "$CONFIG_FILE" | sort -n
}

# 从配置中获取服务信息
get_server_info() {
    local server_num=$1
    grep "^${server_num}|" "$CONFIG_FILE" | awk -F'|' '{print $1,$2,$3,$4}'
}

# 检查服务是否运行
is_server_running() {
    local port=$1
    lsof -i :$port > /dev/null 2>&1
}

# 启动单个服务
start_server() {
    local server_num=$1
    local server_dir="hq-server${server_num}"
    local port=$((BASE_PORT + server_num - 1))
    local wallet_path="${WALLET_BASE}${server_num}${WALLET_EXT}"

    if is_server_running $port; then
        echo -e "${YELLOW}服务 ${server_dir} (端口: ${port}) 已经在运行${NC}"
        return 1
    fi

    if [ ! -d "$server_dir" ]; then
        mkdir -p "$server_dir"
        printf "$ENV_TEMPLATE" "$wallet_path" > "${server_dir}/.env"
        echo -e "${GREEN}创建服务目录 ${server_dir} 并生成.env文件${NC}"
    fi

    # 添加到配置文件
    add_server_to_config "$server_num" "$port" "$wallet_path" "$server_dir"

    cd "$server_dir" || return 1
    nohup $SERVER_BINARY --port $port --miner-ids 1,2 --priority-fee 1000 > server.log 2>&1 &
    cd ..
    
    echo -e "${GREEN}启动服务 ${server_dir} (端口: ${port})${NC}"
    return 0
}

# 停止单个服务
stop_server() {
    local server_num=$1
    local server_info=($(get_server_info $server_num))
    local port=${server_info[1]}

    if [ -z "$port" ]; then
        echo -e "${RED}找不到服务编号 ${server_num} 的配置${NC}"
        return 1
    fi

    if is_server_running $port; then
        kill $(lsof -t -i :$port) 2>/dev/null
        echo -e "${GREEN}停止服务 hq-server${server_num} (端口: ${port})${NC}"
    else
        echo -e "${YELLOW}服务 hq-server${server_num} (端口: ${port}) 未运行${NC}"
    fi
}

# 重启单个服务
restart_server() {
    local server_num=$1
    stop_server $server_num
    sleep 2
    start_server $server_num
}

# 移除单个服务
remove_server() {
    local server_num=$1
    local server_info=($(get_server_info $server_num))
    local server_dir=${server_info[3]}
    
    stop_server $server_num
    
    if [ -d "$server_dir" ]; then
        rm -rf "$server_dir"
        echo -e "${GREEN}已移除服务目录 ${server_dir}${NC}"
    else
        echo -e "${YELLOW}服务目录 ${server_dir} 不存在${NC}"
    fi
    
    remove_server_from_config $server_num
}

# 显示服务状态
show_status() {
    echo -e "\n${BLUE}=== 服务状态 ===${NC}"
    
    if [ ! -s "$CONFIG_FILE" ]; then
        echo -e "${YELLOW}没有配置任何服务${NC}"
        return
    fi
    
    for server_num in $(get_all_servers); do
        local server_info=($(get_server_info $server_num))
        local port=${server_info[1]}
        local server_dir=${server_info[3]}
        
        if [ -z "$port" ]; then
            continue
        fi
        
        if is_server_running $port; then
            echo -e "${GREEN}${server_dir} (端口: ${port}) - 运行中${NC}"
        else
            echo -e "${RED}${server_dir} (端口: ${port}) - 停止${NC}"
        fi
    done
    echo ""
}

# 显示菜单
show_menu() {
    clear
    echo -e "${BLUE}=== HQ Server 管理脚本 ===${NC}"
    echo -e "1. 批量启动服务"
    echo -e "2. 停止单个服务"
    echo -e "3. 重启单个服务"
    echo -e "4. 停止所有服务"
    echo -e "5. 重启所有服务"
    echo -e "6. 列出服务状态"
    echo -e "7. 移除单个服务"
    echo -e "8. 批量移除服务"
    echo -e "0. 退出"
    echo -ne "\n${YELLOW}请选择操作: ${NC}"
}

# 初始化
init_config

# 主循环
while true; do
    show_menu
    read choice
    case $choice in
        1)
            echo -ne "请输入要启动的服务数量: "
            read count
            for ((i=1; i<=count; i++)); do
                start_server $i
                sleep 1
            done
            ;;
        2)
            show_status
            echo -ne "请输入要停止的服务编号: "
            read num
            stop_server $num
            ;;
        3)
            show_status
            echo -ne "请输入要重启的服务编号: "
            read num
            restart_server $num
            ;;
        4)
            for server_num in $(get_all_servers); do
                stop_server $server_num
            done
            ;;
        5)
            for server_num in $(get_all_servers); do
                restart_server $server_num
                sleep 1
            done
            ;;
        6)
            show_status
            ;;
        7)
            show_status
            echo -ne "请输入要移除的服务编号: "
            read num
            remove_server $num
            ;;
        8)
            show_status
            echo -ne "请输入要移除的服务数量: "
            read count
            for ((i=1; i<=count; i++)); do
                remove_server $i
            done
            ;;
        0)
            echo -e "${GREEN}退出脚本${NC}"
            exit 0
            ;;
        *)
            echo -e "${RED}无效选择，请重新输入${NC}"
            ;;
    esac
    
    echo -e "\n${YELLOW}按回车键继续...${NC}"
    read
done
