# 查询实例的审计日志文档示例

该项目为查询MongoDB实例的审计日志。文档示例，该示例**无法在线调试**，如需调试可下载到本地后替换 [AK](https://usercenter.console.aliyun.com/#/manage/ak) 以及参数后进行调试。

## 运行条件

- 下载并解压需要语言的代码;


- 在阿里云帐户中获取您的 [凭证](https://usercenter.console.aliyun.com/#/manage/ak)并通过它替换下载后代码中的 ACCESS_KEY_ID 以及 ACCESS_KEY_SECRET;

- 执行对应语言的构建及运行语句

## 执行步骤

下载的代码包，在根据自己需要更改代码中的参数和 AK 以后，可以在**解压代码所在目录下**按如下的步骤执行

- Node.js
- *Node.js >= 8.x*
```sh
npm install --registry=https://registry.npmmirror.com && node src/client.js
```
## 使用的 API

-  DescribeAuditRecords 查询MongoDB实例的审计日志。文档示例，可以参考：[文档](https://next.api.aliyun.com/document/Dds/2015-12-01/DescribeAuditRecords)

## 返回示例

*实际输出结构可能稍有不同，属于正常返回；下列输出值仅作为参考，以实际调用为准*


- JSON 格式 
```js
{
    "Items": {
        "SQLRecord": [
            {
                "TotalExecutionTimes": 703,
                "Syntax": "{ \"atype\" : \"command\", \"param\" : { \"command\" : \"find\", \"ns\" : \"123.test1\", \"args\" : { \"find\" : \"test1\", \"filter\" : { \"x\" : 1, \"y\" : 2 }, \"shardVersion\" : [ { \"$timestamp\" : { \"t\" : 0, \"i\" : 0 } }, { \"$oid\" : \"000000000000000000000000\" } ], \"$clusterTime\" : { \"clusterTime\" : { \"$timestamp\" : { \"t\" : 1552275017, \"i\" : 2 } }, \"signature\" : { \"hash\" : { \"$binary\" : \"9qfygDs61fKCvdXJqjq+f0zML0E=\", \"$type\" : \"00\" }, \"keyId\" : { \"$numberLong\" : \"6666955498811555841\" } } }, \"$client\" : { \"application\" : { \"name\" : \"MongoDB Shell\" }, \"driver\" : { \"name\" : \"MongoDB Internal Client\", \"version\" : \"3.4.10\" }, \"os\" : { \"type\" : \"Linux\", \"name\" : \"Ubuntu\", \"architecture\" : \"x86_64\", \"version\" : \"16.04\" }, \"mongos\" : { \"host\" : \"rxxxxxx.cloud.cm10:3074\", \"client\" : \"47.xxx.xxx.xx:53854\", \"version\" : \"4.0.0\" } }, \"$configServerState\" : { \"opTime\" : { \"ts\" : { \"$timestamp\" : { \"t\" : 1552275017, \"i\" : 2 } }, \"t\" : { \"$numberLong\" : \"3\" } } }, \"$db\" : \"123\" } }, \"result\": \"OK\" }",
                "HostAddress": "11.xxx.xxx.xxx",
                "ExecuteTime": "2019-03-11T03:30:27Z",
                "ThreadID": "139xxxxxxxx",
                "AccountName": "__system;",
                "DBName": "local;"
            },
            {
                "TotalExecutionTimes": 0,
                "Syntax": "{ \"atype\" : \"createIndex\", \"param\" : { \"ns\" : \"123.test1\", \"indexName\" : \"y_1\", \"indexSpec\" : { \"v\" : 2, \"key\" : { \"y\" : 1 }, \"name\" : \"y_1\", \"ns\" : \"123.test1\" } }, \"result\": \"OK\" }",
                "HostAddress": "",
                "ExecuteTime": "2019-03-11T03:30:06Z",
                "ThreadID": "140xxxxxxxx",
                "AccountName": "__system;",
                "DBName": "local;"
            }
        ]
    },
    "PageNumber": 1,
    "TotalRecordCount": 2,
    "RequestId": "3278BEB8-503B-4E46-8F7E-D26E040C9769",
    "PageRecordCount": 30
}
```
- XML 格式 
```xml
<?xml version="1.0" encoding="UTF-8" ?>
<DescribeAuditRecordsResponse>
	<Items>
		<SQLRecord>
			<TotalExecutionTimes>703</TotalExecutionTimes>
			<Syntax>{ &quot;atype&quot; : &quot;command&quot;, &quot;param&quot; : { &quot;command&quot; : &quot;find&quot;, &quot;ns&quot; : &quot;123.test1&quot;, &quot;args&quot; : { &quot;find&quot; : &quot;test1&quot;, &quot;filter&quot; : { &quot;x&quot; : 1, &quot;y&quot; : 2 }, &quot;shardVersion&quot; : [ { &quot;$timestamp&quot; : { &quot;t&quot; : 0, &quot;i&quot; : 0 } }, { &quot;$oid&quot; : &quot;000000000000000000000000&quot; } ], &quot;$clusterTime&quot; : { &quot;clusterTime&quot; : { &quot;$timestamp&quot; : { &quot;t&quot; : 1552275017, &quot;i&quot; : 2 } }, &quot;signature&quot; : { &quot;hash&quot; : { &quot;$binary&quot; : &quot;9qfygDs61fKCvdXJqjq+f0zML0E=&quot;, &quot;$type&quot; : &quot;00&quot; }, &quot;keyId&quot; : { &quot;$numberLong&quot; : &quot;6666955498811555841&quot; } } }, &quot;$client&quot; : { &quot;application&quot; : { &quot;name&quot; : &quot;MongoDB Shell&quot; }, &quot;driver&quot; : { &quot;name&quot; : &quot;MongoDB Internal Client&quot;, &quot;version&quot; : &quot;3.4.10&quot; }, &quot;os&quot; : { &quot;type&quot; : &quot;Linux&quot;, &quot;name&quot; : &quot;Ubuntu&quot;, &quot;architecture&quot; : &quot;x86_64&quot;, &quot;version&quot; : &quot;16.04&quot; }, &quot;mongos&quot; : { &quot;host&quot; : &quot;rxxxxxx.cloud.cm10:3074&quot;, &quot;client&quot; : &quot;47.xxx.xxx.xx:53854&quot;, &quot;version&quot; : &quot;4.0.0&quot; } }, &quot;$configServerState&quot; : { &quot;opTime&quot; : { &quot;ts&quot; : { &quot;$timestamp&quot; : { &quot;t&quot; : 1552275017, &quot;i&quot; : 2 } }, &quot;t&quot; : { &quot;$numberLong&quot; : &quot;3&quot; } } }, &quot;$db&quot; : &quot;123&quot; } }, &quot;result&quot;: &quot;OK&quot; }</Syntax>
			<HostAddress>11.xxx.xxx.xx</HostAddress>
			<ExecuteTime>2019-03-11T03:30:27Z</ExecuteTime>
			<ThreadID>139xxxxxxxx</ThreadID>
			<AccountName>__system;</AccountName>
			<DBName>local;</DBName>
		</SQLRecord>
		<SQLRecord>
			<TotalExecutionTimes>0</TotalExecutionTimes>
			<Syntax>{ &quot;atype&quot; : &quot;createIndex&quot;, &quot;param&quot; : { &quot;ns&quot; : &quot;123.test1&quot;, &quot;indexName&quot; : &quot;y_1&quot;, &quot;indexSpec&quot; : { &quot;v&quot; : 2, &quot;key&quot; : { &quot;y&quot; : 1 }, &quot;name&quot; : &quot;y_1&quot;, &quot;ns&quot; : &quot;123.test1&quot; } }, &quot;result&quot;: &quot;OK&quot; }</Syntax>
			<HostAddress></HostAddress>
			<ExecuteTime>2019-03-11T03:30:06Z</ExecuteTime>
			<ThreadID>140xxxxxxxx</ThreadID>
			<AccountName>__system;</AccountName>
			<DBName>local;</DBName>
		</SQLRecord>
	</Items>
	<PageNumber>1</PageNumber>
	<TotalRecordCount>2</TotalRecordCount>
	<RequestId>3278BEB8-503B-4E46-8F7E-D26E040C9769</RequestId>
	<PageRecordCount>30</PageRecordCount>
</DescribeAuditRecordsResponse>
```

