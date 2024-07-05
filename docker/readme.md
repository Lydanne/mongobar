https://www.cnblogs.com/evescn/p/16203350.html

MongoDB 3 副本集群（Docker-compose 部署）（单机模式）

资源清单
主机 IP
mongodb 节点 10.0.0.1
软件 版本
docker 20.10.12
docker-compose 1.23.1
mongodb 镜像 5.0.6
一、Docker 安装

1. 使用国内 yum 源

# yum install -y yum-utils device-mapper-persistent-data lvm2

# yum-config-manager --add-repo https://mirrors.aliyun.com/docker-ce/linux/centos/docker-ce.repo

2. 卸载旧版本的 docker

## 如果主机上已经有 docker 存在且不是想要安装的版本，需要先进行卸载。

# yum remove -y docker \

              docker-client \
              docker-client-latest \
              docker-common \
              docker-latest \
              docker-latest-logrotate \
              docker-logrotate \
              docker-selinux \
              docker-engine-selinux \
              docker-engine \
              container*

3. 安装 Docker20.10 版本

# yum -y install docker-ce-20.10.12-3.el7 docker-ce-cli-20.10.12-3.el7 vim

4. 设置镜像加速

# mkdir /etc/docker

# vi /etc/docker/daemon.json

{
"registry-mirrors": ["https://xxxxxxxxx.mirror.aliyuncs.com"]
} 5. 启动 docker

# systemctl start docker

# systemctl enable docker

# systemctl status docker

二、Docker-compose 安装

1. Docker-compose 安装

## github.com 可能访问超时，可以使用下面的获取下载下来后上传服务器即可

# curl -L "https://github.com/docker/compose/releases/download/1.29.2/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose

# curl -k "https://dl.cactifans.com/zabbix_docker/docker-compose" -o /usr/bin/docker-compose

# chmod a+x /usr/bin/docker-compose

2. 查看 docker-compose 版本

# docker-compose version

三、MongoDB 3 副本集群 安装

1. 生成 keyFile
   MongoDB 使用 keyfile 认证，副本集中的每个 mongod 实例使用 keyfile 内容作为认证其他成员的共享密码。mongod 实例只有拥有正确的 keyfile 才可以加入副本集。
   keyFile 的内容必须是 6 到 1024 个字符的长度，且副本集所有成员的 keyFile 内容必须相同。
   有一点要注意是的：在 UNIX 系统中，keyFile 必须没有组权限或完全权限（也就是权限要设置成 X00 的形式）。Windows 系统中，keyFile 权限没有被检查。
   可以使用任意方法生成 keyFile。例如，如下操作使用 openssl 生成复杂的随机的 1024 个字符串。然后使用 chmod 修改文件权限，只给文件拥有者提供读权限。这是 MongoDB 官方推荐 keyFile 的生成方式

## 400 权限是要保证安全性，否则 mongod 启动会报错

# openssl rand -base64 756 > mongodb.key

# chmod 400 mongodb.key

2. 详细的 docker-compose.yml 文件信息
   version: "3"

services: #主节点
mongodb1:
image: mongo:5.0.6
container_name: mongo1
restart: always
ports: - 27017:27017
environment: - MONGO_INITDB_ROOT_USERNAME=root - MONGO_INITDB_ROOT_PASSWORD=mongodb@evescn
command: mongod --replSet rs0 --keyFile /mongodb.key
volumes: - /etc/localtime:/etc/localtime - /data/mongodb/mongo1/data:/data/db - /data/mongodb/mongo1/configdb:/data/configdb - /data/mongodb/mongo1/mongodb.key:/mongodb.key
networks: - mongoNet
entrypoint: - bash - -c - |
chmod 400 /mongodb.key
chown 999:999 /mongodb.key
exec docker-entrypoint.sh $$@

# 副节点

mongodb2:
image: mongo:5.0.6
container_name: mongo2
restart: always
ports: - 27018:27017
environment: - MONGO_INITDB_ROOT_USERNAME=root - MONGO_INITDB_ROOT_PASSWORD=mongodb@evescn
command: mongod --replSet rs0 --keyFile /mongodb.key
volumes: - /etc/localtime:/etc/localtime - /data/mongodb/mongo2/data:/data/db - /data/mongodb/mongo2/configdb:/data/configdb - /data/mongodb/mongo2/mongodb.key:/mongodb.key
networks: - mongoNet
entrypoint: - bash - -c - |
chmod 400 /mongodb.key
chown 999:999 /mongodb.key
exec docker-entrypoint.sh $$@

# 副节点

mongodb3:
image: mongo:5.0.6
container_name: mongo3
restart: always
ports: - 27019:27017
environment: - MONGO_INITDB_ROOT_USERNAME=root - MONGO_INITDB_ROOT_PASSWORD=mongodb@evescn
command: mongod --replSet rs0 --keyFile /mongodb.key
volumes: - /etc/localtime:/etc/localtime - /data/mongodb/mongo3/data:/data/db - /data/mongodb/mongo3/configdb:/data/configdb - /data/mongodb/mongo3/mongodb.key:/mongodb.key
networks: - mongoNet
entrypoint: - bash - -c - |
chmod 400 /mongodb.key
chown 999:999 /mongodb.key
exec docker-entrypoint.sh $$@
networks:
mongoNet:
driver: bridge 3. MongoDB 3 副本集群 部署

# mkdir /data/mongodb/mongo{1,2,3}/{data,configdb} -pv

## 提供 redis.conf 配置

# cp mongodb.key /data/mongodb/mongo1/

# cp mongodb.key /data/mongodb/mongo2/

# cp mongodb.key /data/mongodb/mongo3/

# docker-compose up -d

4. 配置集群
   a | 进入 Mongo 容器链接 Mongo

# 选择第一个容器 mongo1，进入 mongo 容器

docker exec -it mongo1 bash

# 登录 mongo

# mongo -u root -p mongodb@evescn

b | 或者通过以下方式进入 Mongo 容器链接 Mongo

# docker exec -it mongo1 mongo

c | 通过以下指令配置 mongo 副本集集群

# 认证

> use admin
> db.auth('root', 'mongodb@evescn')
> 成功返回 1，失败返回 0

5. 使用配置文件初始化集群
   单主机模式部署 3 副本集 添加节点必须使用宿主机 IP+PORT，
   使用容器内部 IP 的情况下代码层面连接到 mongodb-cluster 集群，
   获取到的集群地址信息为 docker 容器内部 IP，
   若业务代码没有部署在 mongodb 主机则无法访问

a | 配置文件

> config={\_id:"rs0",members:[
>
> > {\_id:0,host:"10.0.0.1:27017"},
> > {\_id:1,host:"10.0.0.1:27018"},
> > {\_id:2,host:"10.0.0.1:27019"}]
> > }
> > b | 初始化集群
> > rs.initiate(config)
> > c | 增长 mongo1 和 mongo2 的权重
> > cfg = rs.conf()

# 修改权重

> cfg.members[0].priority=5
> cfg.members[1].priority=3

# 从新配置

> rs.reconfig(cfg)
> d | 验证副本集

# 切换节点查看同步状态：

> rs.printReplicationInfo()

# 仅当建立了集合后副节点才会进行同步

6. 单节点初始化副本集
   单主机模式部署 3 副本集 添加节点必须使用宿主机 IP+PORT，
   使用容器内部 IP 的情况下代码层面连接到 mongodb-cluster 集群，
   获取到的集群地址信息为 docker 容器内部 IP，
   若业务代码没有部署在 mongodb 主机则无法访问

a | 初始化副本集
mongodb-cluster 集群

> rs.initiate()

## 无参初始化后，当前节点默认是 PRIMARY 节点，

## 并且节点信息为容器主机名+PORT，后续需要删除节点后重新添加到集群中

b | 添加节点

## 副节点

> rs.add({\_id:1,host:"10.0.0.1:27018"})

## 副节点

> rs.add({\_id:2,host:"10.0.0.1:27019"})
> c | 查看副本集配置信息和运行状态

## 查看副本集配置信息

> rs.conf()

## 查看副本集运行状态：

> rs.status()

......
"members" : [
{
"\_id" : 0,
"name" : "d266ffd0e331:27017",
"health" : 1,
"state" : 1,
"stateStr" : "PRIMARY",
......
},
......

d | 暂停节点 1

# docker stop mongo1

e | 找到集群新的主节点，添加节点 1，

# docker start mongo2 mongo

## 认证

> use admin
> db.auth('root', 'mongodb@evescn')
> rs.remove("d266ffd0e331:27017") ## rs.status() 查看集群节点 1 的信息

## 添加节点 1

> rs.add({\_id:0,host:"10.0.0.1:27017"})

f | 增长 mongo1 和 mongo2 的权重

> cfg = rs.conf()

# 修改权重

> cfg.members[0].priority=5
> cfg.members[1].priority=3

# 从新配置

> rs.reconfig(cfg)

g | 验证副本集

# 切换节点查看同步状态：

> rs.printReplicationInfo()

# 仅当建立了集合后副节点才会进行同步

« 上一篇： Redis 1 主 2 从 3 哨兵（Docker-compose 部署）（单机模式）
» 下一篇： Mesos 3 主 3 从（Docker-compose 部署）
posted @ 2022-04-28 16:22 evescn 阅读(2744) 评论(1) 编辑 收藏 举报
登录后才能查看或发表评论，立即 登录 或者 逛逛 博客园首页
编辑推荐：
· 不单独部署注册中心，又要具备注册中心的功能
· ［动画进阶］类 ChatGpt 多行文本打字效果
· 如何找到并快速上手一个开源项目
· [WPF] 用 HtmlTextBlock 实现消息对话框的内容高亮和跳转
· 聊一聊 C# 弱引用 底层是怎么玩的
阅读排行：
· 学习.NET 8 MiniApis 入门
· 全新 UI 震撼来袭！ng-matero v18 正式发布！
· 炎炎夏日，清凉上线：博客园 T 恤丝光棉款上架预售
· 基于 Bootstrap Blazor 开源的.NET 通用后台权限管理系统
· 面试官：你了解 git cherry-pick 吗？
Copyright © 2024 evescn
Powered by .NET 8.0 on Kubernetes
