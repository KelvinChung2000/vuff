module m;
  initial   begin
    z  =  '0;
    z  =  '1;
    z  =  'x;
    z  =  'z;
  end
endmodule
// expected -----
module m;
  initial begin
    z = '0;
    z = '1;
    z = 'x;
    z = 'z;
  end
endmodule
