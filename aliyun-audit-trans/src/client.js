'use strict';
// This file is auto-generated, don't edit it
// 依赖的模块可通过下载工程中的模块依赖文件或右上角的获取 SDK 依赖信息查看
require('dotenv').config();
const OpenApi = require('@alicloud/openapi-client');
const Console = require('@alicloud/tea-console');
const OpenApiUtil = require('@alicloud/openapi-util');
const Util = require('@alicloud/tea-util');
const Tea = require('@alicloud/tea-typescript');
const { writeFile, appendFile } = require('fs');


class Client {

  /**
   * 使用AK&SK初始化账号Client
   * @return Client
   * @throws Exception
   */
  static createClient() {
    // 工程代码泄露可能会导致 AccessKey 泄露，并威胁账号下所有资源的安全性。以下代码示例仅供参考。
    // 建议使用更安全的 STS 方式，更多鉴权访问方式请参见：https://help.aliyun.com/document_detail/378664.html。
    let config = new OpenApi.Config({
      // 必填，请确保代码运行环境设置了环境变量 ALIBABA_CLOUD_ACCESS_KEY_ID。
      accessKeyId: process.env['ALIBABA_CLOUD_ACCESS_KEY_ID'],
      // 必填，请确保代码运行环境设置了环境变量 ALIBABA_CLOUD_ACCESS_KEY_SECRET。
      accessKeySecret: process.env['ALIBABA_CLOUD_ACCESS_KEY_SECRET'],
    });
    // Endpoint 请参考 https://api.aliyun.com/product/Dds
    config.endpoint = `mongodb.aliyuncs.com`;
    return new OpenApi.default(config);
  }

  /**
   * API 相关
   * @param path params
   * @return OpenApi.Params
   */
  static createApiInfo() {
    let params = new OpenApi.Params({
      // 接口名称
      action: 'DescribeAuditRecords',
      // 接口版本
      version: '2015-12-01',
      // 接口协议
      protocol: 'HTTPS',
      // 接口 HTTP 方法
      method: 'POST',
      authType: 'AK',
      style: 'RPC',
      // 接口 PATH
      pathname: `/`,
      // 接口请求体内容格式
      reqBodyType: 'json',
      // 接口响应体内容格式
      bodyType: 'json',
    });
    return params;
  }

  static async load(PageNumber) {
    let client = Client.createClient();
    let params = Client.createApiInfo();
    // query params
    let queries = process.env['QUERIES'] ? JSON.parse(process.env['QUERIES']) : {};
    queries.PageSize = 1000;
    queries.PageNumber = PageNumber;
    // runtime options
    let runtime = new Util.RuntimeOptions({});
    let request = new OpenApi.OpenApiRequest({
      query: OpenApiUtil.default.query(queries),
    });
    // 复制代码运行请自行打印 API 的返回值
    // 返回值为 Map 类型，可从 Map 中获得三类数据：响应体 body、响应头 headers、HTTP 返回的状态码 statusCode。
    let resp = await client.callApi(params, request, runtime);
    // Console.default.log(Util.default.toJSONString(resp));
    const data = resp.body.Items.SQLRecord;
    const OpMap = {
      "find": "Find",
      "update": "Update",
      "count": "Count",
      "getMore": "GetMore",
      "insert": "Insert",
      "delete": "Delete",
      "aggregate": "Aggregate",
      "findAndModify": "FindAndModify",
    }
    let stats = Object.keys(OpMap).reduce((acc, key) => {
      acc[OpMap[key]] = 0;
      return acc;
    }, {})
    data.forEach((item) => {
      // op_row => {"id":"A300CFDE","op":"Query","db":"xgj","coll":"classes","cmd":{"find":"classes","filter":{"_id":{"$in":[{"$oid":"60ee8954fed35014bf22675a"}]}},"projection":{},"$readPreference":{"mode":"secondaryPreferred"}},"ns":"xgj.classes","ts":1720432985163,"st":"None"}
      const Syntax = JSON.parse(item.Syntax);
      const op = OpMap[Syntax.command];
      if (!op) {
        return;
      }
      const cmd = Syntax.args;
      deepTraverseAndConvert(cmd);
      const [db, coll] = Syntax.ns.split('.');
      const op_row = {
        id: md5(JSON.stringify(item)),
        op: op,
        db: item.DBName,
        coll,
        cmd,
        ns: Syntax.ns,
        ts: Date.parse(item.ExecuteTime),
      }
      appendFile('./tmp/oplogs.op', JSON.stringify(op_row) + '\n', (err) => {
        if (err) throw err;
        // console.log('The file has been saved!');
      });
      stats[op] += 1;
    });
    console.log(`The file has been saved ${data.length} rows, stats ${JSON.stringify(stats)}!`);
    return data.length;
  }

  /**
   * 入口函数
   * @param args 命令行参数
   */
  static async main(args) {
    try {
      let i = 1;
      while (true) {
        try {
          const count = await Client.load(i);
          if (count === 0) {
            break;
          }
          await new Promise(resolve => setTimeout(resolve, 5000));
          i++;
        } catch (error) {
          await new Promise(resolve => setTimeout(resolve, 60000));
        }
      }
    } catch (e) {
      Console.default.log(e);
    }
  }

}

exports.Client = Client;
Client.main(process.argv.slice(2));


const crypto = require('crypto');

function md5(content) {
  return crypto.createHash('md5').update(content).digest('hex');
}

function convertToRFC3339(dateStr) {
  const date = new Date(dateStr);
  if (isNaN(date.getTime())) {
    return dateStr;
  }
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  const hours = String(date.getHours()).padStart(2, '0');
  const minutes = String(date.getMinutes()).padStart(2, '0');
  const seconds = String(date.getSeconds()).padStart(2, '0');
  const milliseconds = String(date.getMilliseconds()).padStart(3, '0');

  return `${year}-${month}-${day}T${hours}:${minutes}:${seconds}.${milliseconds}Z`;
}


function deepTraverseAndConvert(obj) {
  // if (typeof obj === 'object' && obj !== null) {
  //   delete obj.lsid;
  //   delete obj.$clusterTime;
  //   delete obj.$db;
  //   delete obj.cursor;
  //   delete obj.cursorId;
  // }
  for (const key in obj) {
    if (typeof obj[key] === 'string' && obj[key].includes('T')) {
      obj[key] = convertToRFC3339(obj[key]);
    } else if (typeof obj[key] === 'object' && obj[key] !== null) {
      deepTraverseAndConvert(obj[key]);
    }
  }
}