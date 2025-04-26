use tonic::transport::Channel;
use crate::proto::vdfs::vdfs_service_client::VdfsServiceClient;
use crate::proto::vdfs::{
    CreateFileRequest, DeleteFileRequest, ReadFileRequest, WriteFileRequest,
    ListDirectoryRequest, GetFileInfoRequest, FileType,
};
use crate::error::Result;
use futures::stream;

pub struct VDFSClient {
    client: VdfsServiceClient<Channel>,
}

impl VDFSClient {
    pub async fn new(addr: String) -> Result<Self> {
        let client = VdfsServiceClient::connect(addr).await?;
        Ok(Self { client })
    }

    pub async fn create_file(&mut self, path: String, node_id: String) -> Result<()> {
        let request = CreateFileRequest {
            path,
            node_id,
            r#type: FileType::File as i32,
        };
        self.client.create_file(request).await?;
        Ok(())
    }

    pub async fn delete_file(&mut self, path: String, node_id: String) -> Result<()> {
        let request = DeleteFileRequest { path, node_id };
        self.client.delete_file(request).await?;
        Ok(())
    }

    pub async fn read_file(&mut self, path: String, node_id: String, offset: i64, length: i64) -> Result<Vec<u8>> {
        let request = ReadFileRequest { path, node_id, offset, length };
        let mut response = self.client.read_file(request).await?.into_inner();
        let mut data = Vec::new();
        while let Some(chunk) = response.message().await? {
            data.extend_from_slice(&chunk.data);
        }
        Ok(data)
    }

    pub async fn write_file(&mut self, path: String, node_id: String, data: Vec<u8>) -> Result<()> {
        let request = stream::iter(vec![WriteFileRequest {
            path,
            node_id,
            data,
            offset: 0,
        }]);
        self.client.write_file(request).await?;
        Ok(())
    }

    pub async fn list_directory(&mut self, path: String, node_id: String) -> Result<Vec<String>> {
        let request = ListDirectoryRequest { path, node_id };
        let response = self.client.list_directory(request).await?;
        Ok(response.into_inner().entries.into_iter().map(|e| e.path).collect())
    }

    pub async fn get_file_info(&mut self, path: String, node_id: String) -> Result<crate::proto::vdfs::FileInfo> {
        let request = GetFileInfoRequest { path, node_id };
        let response = self.client.get_file_info(request).await?;
        Ok(response.into_inner().info.unwrap())
    }
} 