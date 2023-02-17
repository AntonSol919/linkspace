Some ideas on HTTP bridges
A publicly facing server exposing a HTTP bridge should
at least impl the following API at some root 

All linkspace httpbridge should implemented the following : 
## API 
- POST ./view 
  The post data is inerpreted as a query.
- POST ./get   
  This is equal to ./view ++ i_new:=:{u32:0} 
- POST ./get/hash/HASH  
  This helps with cashing
- POST ./save
  Optional endpoint to receiv new packets
- GET ./accepts
  adoto-json encoded 
  double newline seperated set of queries. 
  The ./view will attempt to merge from start to finish with these sets. 
  If one of them is accepted the query is executed. 

