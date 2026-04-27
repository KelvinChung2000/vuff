module m;
  assign  bus[7:0]  =  data[7:0];
  assign  word[31:16]  =  src[15:0];
endmodule
// expected -----
module m;
  assign bus[7:0] = data[7:0];
  assign word[31:16] = src[15:0];
endmodule
