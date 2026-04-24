module   test  ;
  assign  a   =  b  +  c  ;
endmodule
// expected -----
module test;
  assign a = b + c;
endmodule
